use crate::core::error::exception::rhai_error::RhaiError;
use crate::core::rhai::query_planner::QueryPlanner;
use rhai::{AST, Dynamic, Engine, FnPtr};
use tracing::{debug, error};

/// The RHAI parser parses the script and converts it into a QueryPlan
/// that RUST can natively execute instead of RHAI runtime
pub struct RhaiParser {
    engine: Engine,
}

impl RhaiParser {
    pub fn new(engine: Engine) -> Self {
        Self { engine }
    }

    fn parse_script(&self, script: &str) -> Result<AST, RhaiError> {
        self.engine
            .compile(script)
            .map_err(|e| RhaiError::ScriptCompilation(e.to_string()))
    }

    fn evaluate_ast(&self, ast: &AST) -> Result<rhai::Dynamic, RhaiError> {
        self.engine
            .eval_ast(ast)
            .map_err(|e| RhaiError::ScriptCompilation(e.to_string()))
    }

    fn compile_to_fn_ptr(&self, dynamic: Dynamic) -> Result<Vec<FnPtr>, RhaiError> {
        let array: Vec<Dynamic> = dynamic
            .try_cast()
            .ok_or_else(|| RhaiError::TypeMismatch("Expected array from Rhai script".into()))?;

        let fns = array
            .into_iter()
            .map(|d| {
                d.try_cast::<FnPtr>()
                    .ok_or_else(|| RhaiError::TypeMismatch("Expected FnPtr".into()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(fns)
    }

    fn execute_fn_ptrs(&self, ast: &AST, fns: Vec<FnPtr>) -> Result<Vec<QueryPlanner>, RhaiError> {
        let mut plans = Vec::with_capacity(fns.len());

        for f in fns {
            match f.call::<QueryPlanner>(&self.engine, ast, ()) {
                Ok(plan) => {
                    debug!(?plan, "Generated query plan");
                    plans.push(plan);
                }
                Err(e) => {
                    error!(error = %e, "Failed to execute closure");
                    return Err(RhaiError::ScriptCompilation(e.to_string()));
                }
            }
        }
        Ok(plans)
    }

    pub fn parse(&self, script: &str) -> Result<Vec<QueryPlanner>, RhaiError> {
        let ast = self.parse_script(script)?;
        let dynamic = self.evaluate_ast(&ast)?;
        let fns = self.compile_to_fn_ptr(dynamic)?;
        self.execute_fn_ptrs(&ast, fns)
    }
}
