use rhai::{Array, Dynamic, Engine, Scope};
use std::sync::Arc;
use std::time::Instant;
fn sum_slice(data: &[i64]) -> i64 {
    data.iter().sum()
}

fn sum_list_rhai(data: &mut Array) -> i64 {
    data.iter().map(|v| v.as_int().unwrap_or(0)).sum()
}

fn sum_arc_vec(data: Arc<Vec<i64>>) -> i64 {
    data.iter().sum()
}

#[test]
fn sum_of_list() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Engine::new();

    // This is our data in Rust
    let my_data = vec![10, 20, 30, 40];
    let mut array_rhai = Array::new();
    array_rhai.push(Dynamic::from_int(10));
    array_rhai.push(Dynamic::from_int(20));
    array_rhai.push(Dynamic::from_int(30));
    array_rhai.push(Dynamic::from_int(40));

    // Create a scope to pass variables to Rhai
    let mut scope = Scope::new();
    scope.push_constant("data", array_rhai.clone());

    // Register Rust function with Rhai
    let mut engine = engine;
    engine.register_fn("sum_list", sum_list_rhai);

    // Rhai script decides the "recipe"
    let script = r#"
        let total = sum_list(data);
        print(`Sum is: ${total}`);
        total
    "#;

    // Run the script
    let result: i64 = engine.eval_with_scope(&mut scope, script)?;
    println!("Result from Rhai: {}", result);

    Ok(())
}

#[test]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = Engine::new();

    use rayon::prelude::*;
    use std::sync::Arc;

    let n = 1_000_000;
    let rust_data: Arc<Vec<i64>> = Arc::new((1..=n as i64).into_par_iter().collect());

    let mut scope = Scope::new();
    scope.push_constant("data", Dynamic::from(rust_data.clone()));

    // Register Rust function with Rhai
    engine.register_fn("sum_list", sum_arc_vec); // Rhai script decides the operation
    let script = r#"
        let total = sum_list(data);
        print(`Sum is: ${total}`);
        total
    "#;

    let start = Instant::now();
    let result: i64 = engine.eval_with_scope(&mut scope, script)?;
    let duration = start.elapsed();
    println!("Rhai + Rust result: {:?}", result);
    println!("Time taken: {:.2?}", duration);

    Ok(())
}
