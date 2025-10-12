use crate::api::http::messages::metric_message::{AggregationFn, FilterExpr, MetricRule, Scope};
use crate::application::actors::rhai::rhai_actor::RhaiActor;
use actix::{Handler, Message};

#[derive(Message, Clone, Debug)]
#[rtype(result = "()")]
pub struct MetricRuleDTO {
    pub(crate) flight_name: String,
    pub(crate) rule_name: String,
    pub(crate) scope: Scope,
    pub(crate) window_sec: u16,
    pub(crate) filter: FilterExpr,
    pub(crate) group_by: Vec<String>,
    pub(crate) aggregations: Vec<AggregationFn>,
}

impl From<MetricRule> for MetricRuleDTO {
    fn from(value: MetricRule) -> Self {
        let metric_rule = MetricRuleDTO {
            flight_name: value.get_flight_name().parse().unwrap(),
            rule_name: value.get_rule_name().parse().unwrap(),
            scope: value.get_scope().clone(),
            window_sec: value.get_window_sec(),
            filter: value.get_filter().clone(),
            group_by: value.get_group_by().clone(),
            aggregations: value.get_aggregations().clone(),
        };
        metric_rule
    }
}

impl Handler<MetricRuleDTO> for RhaiActor {
    type Result = ();

    fn handle(&mut self, msg: MetricRuleDTO, _ctx: &mut Self::Context) -> Self::Result {
        let meter_rule = MetricRuleDTO::from(msg.clone());
        self.rules.insert(msg.rule_name, meter_rule.into());
    }
}

impl MetricRuleDTO {
    pub fn get_flight_name(&self) -> &str {
        &self.flight_name
    }
}
