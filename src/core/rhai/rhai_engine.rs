pub(crate) use crate::core::rhai::engine_builder::execution_engine;
use crate::core::rhai::query_planner::QueryPlanner;
use rhai::Engine;
use std::collections::HashMap;

pub struct RhaiEngine {
    pub engine: Engine,
}

impl RhaiEngine {
    pub fn new(engine: Engine) -> Self {
        Self { engine }
    }

    pub fn execute(&self) -> QueryPlanner {
        let script = r#"
            let q = query();
            q.filter(eq_int("service_id", 42));     // specific service
            q.filter(gt_int("cpu_usage", 80));      // cpu > 80
            q.agg("max", "cpu_usage");              // max CPU per window
            q.limit(10);
            q
        "#;

        // compile & run it
        let engine = execution_engine();
        let rhai_engine = RhaiEngine::new(engine);
        let mut scope = rhai::Scope::new();
        let plan: QueryPlanner = rhai_engine
            .engine
            .eval_with_scope(&mut scope, script)
            .unwrap();
        println!("QueryPlan = {:?}", plan);
        plan
    }
}

pub type Record = HashMap<String, i64>;
