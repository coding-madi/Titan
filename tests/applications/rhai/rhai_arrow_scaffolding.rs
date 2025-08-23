use arrow_array::{Int64Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use rhai::{Dynamic, Engine, Map, Scope};
use std::sync::Arc;

#[test]
fn arrow_rhai_scaffolding_only() -> Result<(), Box<dyn std::error::Error>> {
    // 1️⃣ Define Arrow schema & data
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, false),
        Field::new("value", DataType::Int64, false),
    ]));

    let id_col = Arc::new(Int64Array::from(vec![1, 2, 3])) as _;
    let name_col = Arc::new(StringArray::from(vec!["a", "b", "c"])) as _;
    let value_col = Arc::new(Int64Array::from(vec![10, 20, 30])) as _;

    let batch = RecordBatch::try_new(schema.clone(), vec![id_col, name_col, value_col])?;

    // 2️⃣ Only push schema metadata — no column data
    let col_names: Vec<String> = batch
        .schema()
        .fields()
        .iter()
        .map(|f| f.name().clone())
        .collect();

    let mut scope = Scope::new();
    scope.push_constant("columns", col_names.clone());

    // 3️⃣ Rhai script — purely a logical plan, no actual computation
    let script = r#"
        [
            #{ op: "sum", column: "value", filter: "id > 1" },
            #{ op: "count", column: "name" }
        ]
    "#;

    // 4️⃣ Evaluate to get logic description
    let engine = Engine::new();
    let rules_dynamic: Dynamic = engine.eval_with_scope(&mut scope, script)?;
    println!("Type tag: {:?}", rules_dynamic.type_name());
    println!("Value debug: {:?}", rules_dynamic);
    println!("Top-level type: {}", rules_dynamic.type_name());

    if let Some(arr) = rules_dynamic.clone().try_cast::<Vec<Dynamic>>() {
        for (i, item) in arr.iter().enumerate() {
            println!("Element {}: {}", i, item.type_name());
        }
    }

    let arr: Vec<Dynamic> = rules_dynamic
        .clone()
        .try_cast()
        .ok_or("Expected array from Rhai script")?;

    let rules: Vec<Map> = arr
        .into_iter()
        .map(|dyn_val| dyn_val.try_cast::<Map>().unwrap())
        .collect();

    // let rules: Vec<Map> = rules_dynamic
    //     .try_cast()
    //     .ok_or("Expected Vec<Map> from Rhai script")?; // ✅ no unwrap panic

    println!("Declared rules (no computation yet): {:?}", rules);

    for rule in &rules {
        let op: String = rule
            .get("op")
            .and_then(|v| v.clone().try_cast())
            .unwrap_or_default();

        let column: String = rule
            .get("column")
            .and_then(|v| v.clone().try_cast())
            .unwrap_or_default();

        let filter: Option<String> = rule.get("filter").and_then(|v| v.clone().try_cast());

        println!(
            "Executing {} on column {} with filter {:?}",
            op, column, filter
        );
    }

    Ok(())
}
