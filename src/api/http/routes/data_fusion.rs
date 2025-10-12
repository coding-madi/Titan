use crate::application::actors::iceberg::iceberg_actor::{GetBuffer, IcebergActorAddr};
use crate::monitor::prometheus::registry::JOB_LATENCY_HISTOGRAM;
use crate::platform::registry::{FetchIcebergActor, Registry};
use actix::Addr;
use actix_web::web::Data;
use actix_web::{Resource, web};
use arrow_array::RecordBatch;
use datafusion::catalog::MemTable;
use datafusion::prelude::*;
use futures_util::SinkExt;
use serde_derive::Deserialize;
use std::io::Error;
use std::sync::Arc;
use tracing::{error, info};

pub fn execute_sql_factory() -> Resource {
    web::resource("/sql").route(web::post().to(execute))
}

#[derive(Deserialize)]
pub struct ExecuteSql {
    stream_name: String,
    sql: String,
}

pub async fn execute(
    execute_sql: web::Json<ExecuteSql>,
    registry: Data<Arc<Addr<Registry>>>,
) -> Result<String, Error> {
    let _timer = JOB_LATENCY_HISTOGRAM
        .with_label_values(&[
            execute_sql.stream_name.to_string(),
            "sql_query_latencies".to_string(),
        ])
        .start_timer();
    if let Ok(Ok(iceberg_actor)) = registry.send(FetchIcebergActor).await {
        info!("Iceberg Actor found");
        let ctx = SessionContext::new();
        match iceberg_actor {
            IcebergActorAddr::Real(iceberg_actor) => {
                let buffer = match iceberg_actor
                    .send(GetBuffer::new(execute_sql.stream_name.clone()))
                    .await
                {
                    Ok(inner) => match inner {
                        Ok(buf) => buf,
                        Err(e) => {
                            error!("Actor returned error: {:?}", e);
                            panic!("Actor returned error");
                        }
                    },
                    Err(mailbox_err) => {
                        error!("Mailbox error: {:?}", mailbox_err);
                        panic!("Mailbox overflow or actor stopped");
                    }
                };

                let all_batches: Vec<RecordBatch> = buffer
                    .iter()
                    .map(|w| w.get_data().as_ref().clone()) // extract the RecordBatch from wrapper
                    .collect();

                let registered_table =
                    MemTable::try_new(all_batches[0].schema(), vec![all_batches])?;

                ctx.register_table(execute_sql.stream_name.clone(), Arc::new(registered_table))?;

                let df = ctx.sql(&execute_sql.sql).await;

                match df {
                    Ok(df) => {
                        let results = df.collect().await?;
                        let formatted = arrow::util::pretty::pretty_format_batches(&results)
                            .unwrap()
                            .to_string();
                        Ok(formatted)
                    }
                    Err(e) => {
                        println!("{:?}", e);
                        unimplemented!()
                    }
                }
            }

            #[cfg(test)]
            IcebergActorAddr::Mock(_actor) => {
                // let x = actor.send(GetBuffer).await;
                unimplemented!()
            }
            IcebergActorAddr::Empty => {
                panic!("No Iceberg Actor");
            }
        }
    } else {
        panic!("No Registry");
    }
}
