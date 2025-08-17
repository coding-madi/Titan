use crate::core::metric::rhai_engine::RhaiEngine;
use tracing::debug;

#[derive(Debug, Clone)]
pub enum Operator {
    Eq,
    NotEq,
    Gt,
    Gte,
    Lt,
    Lte,
}

#[derive(Debug, Clone)]
pub enum FilterValue {
    Int(i64),
    String(String),
}

#[derive(Debug, Clone)]
pub struct Filter {
    pub column: String,
    pub op: Operator,
    pub value: FilterValue,
}

#[derive(Debug, Clone)]
pub struct QueryPlan {
    pub filters: Vec<Filter>,
    pub aggregates: Vec<(String, String)>, // (op, column)
    pub window: Option<usize>,
    pub limit: Option<usize>,
}

impl QueryPlan {
    pub fn new() -> Self {
        Self {
            filters: vec![],
            aggregates: vec![],
            window: None,
            limit: None,
        }
    }

    pub fn build_query_plan(engine: &RhaiEngine, script: &str) -> Vec<QueryPlan> {
        let engine = &engine.engine;
        // compile the script into ASTs
        let ast = engine.compile(script).unwrap();
        debug!("Parsed AST - {:?}", ast);
        let closures_dynamic: rhai::Dynamic = engine.eval_ast(&ast).unwrap();

        let closures_vec: Vec<rhai::Dynamic> = closures_dynamic
            .try_cast()
            .expect("Expected array from Rhai script");

        let closures: Vec<rhai::FnPtr> = closures_vec
            .into_iter()
            .map(|d| d.try_cast::<rhai::FnPtr>().unwrap())
            .collect();

        let mut plans: Vec<QueryPlan> = vec![];

        for c in closures {
            let plan = c.call(&engine, &ast, ()).unwrap();
            debug!("Plan - {:?}", plan);
            plans.push(plan);
        }
        plans
    }
}

impl QueryPlan {

    pub fn filter(mut self, f: Filter) -> Self {
        self.filters.push(f);
        self
    }

    pub fn agg(mut self, op: &str, col: &str) -> Self {
        self.aggregates.push((op.into(), col.into()));
        self
    }

    pub fn window(mut self, size: i64) -> Self {
        self.window = Some(size as usize);
        self
    }

    pub fn limit(mut self, n: i64) -> Self {
        self.limit = Some(n as usize);
        self
    }

    pub fn eq_int(col: &str, val: i64) -> Filter {
        Filter { column: col.into(), op: Operator::Eq, value: FilterValue::Int(val) }
    }

    pub fn gt_int(col: &str, val: i64) -> Filter {
        Filter { column: col.into(), op: Operator::Gt, value: FilterValue::Int(val) }
    }
}

#[async_trait::async_trait]
pub trait Parser {
    async fn parse(&self) -> QueryPlan;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::metric::rhai_engine::RhaiEngine;

    #[test]
    fn test_query_plan() {
        let engine = RhaiEngine::new().engine;

        let script = r#"
            [
                || query()
                    .filter(gt_int("age", 30))
                    .agg("sum", "sales")
                    .window(7)
                    .limit(100),

                || query()
                    .agg("avg", "salary")
            ]
            "#;
        let ast = engine.compile(script).unwrap();
        let closures_dynamic: rhai::Dynamic = engine.eval_ast(&ast).unwrap();

        let closures_vec: Vec<rhai::Dynamic> = closures_dynamic
            .try_cast()
            .expect("Expected array from Rhai script");

        let closures: Vec<rhai::FnPtr> = closures_vec
            .into_iter()
            .map(|d| d.try_cast::<rhai::FnPtr>().unwrap())
            .collect();
        // let closures: Vec<rhai::FnPtr> = engine.eval_ast(&ast).unwrap();
        let mut plans = Vec::new();

        for c in closures {
            let plan: QueryPlan = c.call(&engine, &ast, ()).unwrap();
            plans.push(plan);
        }

        println!("{:#?}", plans);

    }

}