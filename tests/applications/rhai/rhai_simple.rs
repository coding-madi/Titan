#[cfg(test)]
mod tests {
    use rhai::{Engine, EvalAltResult};
    use std::error::Error;

    #[test]
    fn test_simple_rhai_script() -> Result<(), Box<dyn Error>> {
        // Arrange: create a Rhai engine
        let mut engine = Engine::new();

        // (Optional) Register a Rust function to call from Rhai
        engine.register_fn("add", |a: i64, b: i64| a + b);

        // Act: write the Rhai script you want to tests
        let script = r#"
            let x = 40;
            let y = 2;
            add(x, y)    // calls our Rust function
        "#;

        // Execute the script
        let result: i64 = engine.eval(script)?;

        // Assert: verify the result
        assert_eq!(result, 42);

        Ok(())
    }

    #[test]
    fn test_rhai_with_condition() -> Result<(), Box<dyn Error>> {
        let engine = Engine::new();

        let script = r#"
            let score = 85;
            if score >= 50 {
                "Pass"
            } else {
                "Fail"
            }
        "#;

        let result: String = engine.eval(script)?;
        assert_eq!(result, "Pass");

        Ok(())
    }
}
