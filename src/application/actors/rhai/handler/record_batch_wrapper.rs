use std::cmp::max;
use std::sync::Arc;
use std::time::Duration;


use actix::{Addr, AsyncContext, Handler, Message, WrapFuture};
use actix_rt::spawn;
use arrow_array::RecordBatch;
use arrow_schema::Schema;
use futures_util::SinkExt;
use tracing::{error, info, warn};


use crate::api::http::messages::metric_message::{AggregationFn, StringOrCast};
use crate::application::actors::broadcaster::broadcast_actor::{Metadata, RecordBatchWrapper};
use crate::application::actors::iceberg::iceberg_actor::IcebergActorAddr;
use crate::application::actors::rhai::handler::flush_trigger::FlushTrigger;
use crate::application::actors::rhai::rhai_actor::{RhaiActor, RhaiActorAddr};
use crate::application::actors::wal::metric::handler::record_batch::MetricRecordBatch;
use crate::application::actors::wal::metric::metric_wal_actor::WalMetricActorWrapper;
use crate::core::rhai::planner::filter::FilterOperator;
use crate::core::rhai::query_planner::{AggregateOperation, QueryPlanner};
use crate::platform::registry::{FetchIcebergMeterActor, FetchWalActor, FetchWalMetricActor, Registry};


#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct RhaiActorReady {
    pub(crate) flight_name: String,
    pub rhai_actor_addr: RhaiActorAddr,
}

impl Handler<RecordBatchWrapper> for RhaiActor {
    type Result = ();

    fn handle(&mut self, record_batch_wrapper: RecordBatchWrapper, ctx: &mut Self::Context) -> Self::Result {
        // Safely extract event-time bounds
        let (oldest_ts, latest_ts) = match (
            record_batch_wrapper.get_metadata().oldest_timestamp,
            record_batch_wrapper.get_metadata().newest_timestamp,
        ) {
            (Some(o), Some(l)) => (o, l),
            _ => return, // missing timestamps -> ignore
        };


        // Evaluate rules (modularized)
        for (_id, (rule_name, metric_rule)) in self.rules.iter().enumerate() {
            if let Err(e) = self.process_rule(rule_name, metric_rule, record_batch_wrapper.clone(), ctx) {
                error!("error processing rule {}: {:?}", rule_name, e);
            }
        }


        // Window bookkeeping & scheduling
        let round_down_oldest = self.round_down_to_window(oldest_ts);
        let round_down_latest = self.round_down_to_window(latest_ts);


        if latest_ts <= self.window_state.watermark {
            // late event - drop for now
            return;
        }


        self.window_state.watermark = max(self.window_state.watermark, round_down_oldest);


        self.window_state
            .window_state
            .entry(round_down_oldest)
            .or_insert_with(Vec::new)
            .push(record_batch_wrapper.clone());


        if round_down_latest > round_down_oldest {
            self.window_state
                .window_state
                .entry(round_down_latest)
                .or_insert_with(Vec::new)
                .push(record_batch_wrapper.clone());


            let purge_window = round_down_oldest;
            let new_watermark = round_down_latest;

            // let registry_address = self.registry_address.clone();
            // spawn(async move {
            //     let iceberg_meter_actor_resp = registry_address.send(FetchIcebergMeterActor {}).await;
            //     if let Ok(Ok(iceberg_meter_actor)) = iceberg_meter_actor_resp {
            //         match iceberg_meter_actor {
            //             IcebergActorAddr::Real(iceberg_meter_actor_addr) => {
            //                 iceberg_meter_actor_addr.do_send(record_batch_wrapper.clone());
            //             }
            //             #[cfg(test)]
            //             IcebergActorAddr::Mock(_) => {}
            //             IcebergActorAddr::Empty => {}
            //         }
            //     }
            //
            // });

            if self.window_state.scheduled_windows.insert(purge_window) {
                let grace = self.window_state.grace_period_sec;
                ctx.run_later(Duration::from_millis(grace as u64), move |_act, ctx| {
                    // let registry_address = self.registry_address.send(FetchIcebergMeterActor).await;
                    let rhai_address = ctx.address();
                    rhai_address.do_send(FlushTrigger { purge_window, new_watermark });
                });
            }
        }
    }
}

// -- Helper implementations added to RhaiActor --
impl RhaiActor {
    /// Process a single rule: plan query, evaluate, and forward to WAL actor asynchronously.
    fn process_rule(
        &self,
        rule_name: &str,
        metric_rule: &crate::api::http::messages::metric_message::MetricRule,
        record_batch_wrapper: RecordBatchWrapper,
        ctx: &mut <RhaiActor as actix::Actor>::Context,
    ) -> Result<(), anyhow::Error> {
        let filter = metric_rule.get_filter().clone();
        let filter_operator: FilterOperator = filter.into();
        let group_by_columns = metric_rule.get_group_by().clone();
        let aggregations = metric_rule.get_aggregations().clone();


        // Log the parsed filter for debugging
        match filter_operator.clone() {
            FilterOperator::Condition(condition) => {
                info!("rule={} condition op={:?} value={:?} col={}", rule_name, condition.op, condition.value, condition.column);
            }
            FilterOperator::And(and) => info!("rule={} AND: {:?}", rule_name, and),
            FilterOperator::Or(or) => info!("rule={} OR: {:?}", rule_name, or),
        }


        // Build query planner
        let mut query_planner = QueryPlanner::new().filter(filter_operator).group_by(group_by_columns);


        for aggregation in aggregations.clone() {
            let (aggregation_operation, column): (AggregateOperation, String) = (&aggregation).into();
            query_planner = query_planner.agg(aggregation_operation, &column);
        }


        // Defensive construction of aggregation tuple list for logging/inspection
        let aggregation_list = build_aggregations(&aggregations);
        info!("rule={} Aggregations={:?}", rule_name, aggregation_list);


        // Evaluate query (may return error)
        let metrics_buffer = match self.rhai_service.evaluate(query_planner, record_batch_wrapper.clone()) {
            Ok(buf) => buf,
            Err(e) => {
                error!("rule={} evaluation failed: {:?}", rule_name, e);
                return Err(anyhow::Error::new(e));
            }
        };


        // Capture things needed in async block
        let table_name = "metric".to_string();
        let schema = metrics_buffer.schema();
        let buffer_clone = metrics_buffer.clone();
        let registry = self.registry_address.clone();
        spawn(async move {
            let iceberg_actor_resp = registry.clone().send(FetchIcebergMeterActor).await;

            if let Ok(Ok(iceberg_meter_actor)) = iceberg_actor_resp {
                match iceberg_meter_actor {
                    IcebergActorAddr::Real(iceberg_meter_address) => {
                        iceberg_meter_address.do_send(record_batch_wrapper.clone());
                    }
                    #[cfg(test)]
                    IcebergActorAddr::Mock(_) => {}
                    IcebergActorAddr::Empty => {}
                }
            }
        });

        let registry = self.registry_address.clone();

        // Spawn an async future to fetch WAL actor and send MetricRecordBatch
        ctx.spawn(
            async move {
                spawn_send_to_wal(registry, table_name, schema.as_ref().clone(), buffer_clone).await;
            }
                .into_actor(self),
        );


        Ok(())
    }
}

/// Convert API AggregationFn into a vector of (AggregateOperation, column-name) for logging
fn build_aggregations(aggregations: &[AggregationFn]) -> Vec<(AggregateOperation, String)> {
    aggregations
        .iter()
        .map(|agg_fn| match agg_fn {
            AggregationFn::Count(optional_count_fn) => match optional_count_fn {
                Some(count_fn) => (AggregateOperation::Count, count_fn.to_string()),
                None => (AggregateOperation::Count, "count".to_string()),
            },
            AggregationFn::Sum(sum_fn) => {
                let col_name = match sum_fn {
                    StringOrCast::Field(col_name) => col_name.to_string(),
                    StringOrCast::CastToInt(col_name_with_int_cast) => col_name_with_int_cast.to_string(),
                    StringOrCast::CastToFloat(col_name_with_float_cast) => col_name_with_float_cast.clone(),
                };
                (AggregateOperation::Sum, col_name)
            }
            AggregationFn::Avg(avg_fn) => {
                let col_name = match avg_fn {
                    StringOrCast::Field(col_name) => col_name.to_string(),
                    StringOrCast::CastToInt(col_name_with_int_cast) => col_name_with_int_cast.to_string(),
                    StringOrCast::CastToFloat(col_name_with_float_cast) => col_name_with_float_cast.clone(),
                };
                (AggregateOperation::Avg, col_name)
            }
            AggregationFn::Max(max_fn) => {
                let col_name = match max_fn {
                    StringOrCast::Field(col_name) => col_name.to_string(),
                    StringOrCast::CastToInt(col_name_with_int_cast) => col_name_with_int_cast.to_string(),
                    StringOrCast::CastToFloat(col_name_with_float_cast) => col_name_with_float_cast.clone(),
                };
                (AggregateOperation::Max, col_name)
            }
            AggregationFn::Min(min_fn) => {
                let col_name = match min_fn {
                    StringOrCast::Field(col_name) => col_name.to_string(),
                    StringOrCast::CastToInt(col_name_with_int_cast) => col_name_with_int_cast.to_string(),
                    StringOrCast::CastToFloat(col_name_with_float_cast) => col_name_with_float_cast.clone(),
                };
                (AggregateOperation::Min, col_name)
            }
        })
        .collect()
}

/// Async helper that resolves WAL actor from registry and sends a MetricRecordBatch (fire-and-forget)
async fn spawn_send_to_wal(mut registry: Addr<Registry>, table_name: String, schema: Schema, buffer_clone: Arc<RecordBatch>) {
    match registry.send(FetchWalMetricActor).await {
        Ok(Ok(wal_wrapper)) => match wal_wrapper {
            WalMetricActorWrapper::Real(wal_actor) => {
                let metadata = Metadata::new(&table_name, 0, Arc::new(schema), Some(0), None);
                let metric_batch = MetricRecordBatch::new(metadata, buffer_clone.clone());
                wal_actor.do_send(metric_batch);
            }
            #[cfg(test)]
            WalMetricActorWrapper::Mock(_) => {
                warn!("WAL mock not implemented");
            }
            WalMetricActorWrapper::Empty => {
                warn!("No WAL actor found");
            }
        },
        Ok(Err(err)) => {
            error!("Failed fetching WAL actor: {:?}", err);
        }
        Err(mailbox_err) => {
            error!("WAL actor mailbox error: {:?}", mailbox_err);
        }
    }
}