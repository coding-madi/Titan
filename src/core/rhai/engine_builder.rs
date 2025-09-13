use crate::core::rhai::query_planner::{Filter, QueryPlanner};
use rhai::{Array, Engine};

/// Engine with all the user-defined types and functions registered
pub struct EngineBuilder {
    engine: Engine,
}

impl EngineBuilder {
    fn new() -> Self {
        Self {
            engine: Engine::new(),
        }
    }

    fn with_filter(mut self) -> Self {
        self.engine.register_type_with_name::<Filter>("Filter");

        self.engine
            .register_fn("filter", |q: QueryPlanner, f: Filter| -> QueryPlanner {
                q.filter(f)
            });

        self.engine.register_fn("and", |filters: Array| -> Filter {
            let fs: Vec<Filter> = filters.into_iter().map(|d| d.cast::<Filter>()).collect();
            Filter::And(fs)
        });

        self.engine.register_fn("or", |filters: Array| -> Filter {
            let fs: Vec<Filter> = filters.into_iter().map(|d| d.cast::<Filter>()).collect();
            Filter::Or(fs)
        });
        self
    }

    fn comparison_operator(mut self) -> Self {
        self.engine.register_fn("eq_int", QueryPlanner::eq_int);
        self.engine.register_fn("gt_int", QueryPlanner::gt_int);
        self.engine.register_fn("lt_int", QueryPlanner::lt_int);
        self.engine.register_fn("like", QueryPlanner::like);
        self
    }

    fn with_query_plan(mut self) -> Self {
        self.engine
            .register_type_with_name::<QueryPlanner>("QueryPlanner");

        self.engine.register_fn("query", || QueryPlanner::new());



        self.engine.register_fn(
            "agg",
            |q: QueryPlanner, op: &str, col: &str| -> QueryPlanner { q.agg_str(op, col) },
        );

        self.engine
            .register_fn("group_by", |q: QueryPlanner, cols: Array| -> QueryPlanner {
                let col_names: Vec<String> = cols.into_iter().map(|d| d.cast::<String>()).collect();
                q.group_by(col_names)
            });

        self.engine
            .register_fn("window", |q: QueryPlanner, size: i64| -> QueryPlanner {
                q.window(size)
            });

        self.engine
            .register_fn("limit", |q: QueryPlanner, n: i64| -> QueryPlanner {
                q.limit(n)
            });

        self
    }

    fn build(self) -> Engine {
        self.engine
    }
}

pub fn execution_engine() -> Engine {
    EngineBuilder::new()
        .with_filter()
        .comparison_operator()
        .with_query_plan().build()
}
