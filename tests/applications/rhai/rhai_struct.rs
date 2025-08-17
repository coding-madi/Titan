use rhai::{Engine};

#[derive(Debug, Clone)]
pub struct MyConfig {
    threshold: i64,
}

impl MyConfig {
    pub fn new(threshold: i64) -> Self {
        Self { threshold }
    }

    // Getter
    pub fn get_threshold(&mut self) -> i64 {
        self.threshold
    }

    // Setter
    pub fn set_threshold(&mut self, value: i64) {
        self.threshold = value;
    }

    // Some other function
    pub fn describe(&mut self) -> String {
        format!("Threshold is {}", self.threshold)
    }
}

#[test]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = Engine::new();

    engine
        .register_type::<MyConfig>()
        .register_get("threshold", MyConfig::get_threshold)
        .register_set("threshold", MyConfig::set_threshold)
        .register_fn("describe", MyConfig::describe);

    let mut scope = rhai::Scope::new();
    scope.push("cfg", MyConfig::new(42));

    let script = r#"
        print(cfg.describe());
        cfg.threshold = 100;
        print(cfg.describe());
    "#;

    engine.run_with_scope(&mut scope, script)?;

    Ok(())
}
