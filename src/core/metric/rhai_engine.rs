use crate::core::metric::query_planner::{Filter, QueryPlan};
use rhai::Engine;

fn execution_engine() -> Engine {
    let mut engine = Engine::new();

    engine.register_type_with_name::<Filter>("Filter");
    engine.register_type_with_name::<QueryPlan>("QueryPlan");
    engine.register_fn("query", || QueryPlan::new());
    engine.register_fn("filter", QueryPlan::filter);
    engine.register_fn("agg", QueryPlan::agg);
    engine.register_fn("window", QueryPlan::window);
    engine.register_fn("limit", QueryPlan::limit);

    engine.register_fn("eq_int", QueryPlan::eq_int);
    engine.register_fn("gt_int", QueryPlan::gt_int);

    engine
}

pub struct RhaiEngine {
    pub engine: Engine,
}

impl RhaiEngine {
    pub fn new() -> Self {
        Self {
            engine: execution_engine(),
        }
    }
}
