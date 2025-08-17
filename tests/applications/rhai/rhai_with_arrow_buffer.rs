use std::sync::Arc;
use arrow_array::{Int64Array, StringArray, RecordBatch};
use arrow_schema::{DataType, Field, Schema};
use rhai::{Engine, Scope, Dynamic};

#[test]
fn arrow_rhai_example() -> Result<(), Box<dyn std::error::Error>> {
    // 1️⃣ Define Arrow schema
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, false),
        Field::new("value", DataType::Int64, false),
    ]));

    // 2️⃣ Create Arrow columns
    let id_col = Arc::new(Int64Array::from(vec![1, 2, 3])) as Arc<dyn arrow_array::Array>;
    let name_col = Arc::new(StringArray::from(vec!["a", "b", "c"])) as Arc<dyn arrow_array::Array>;
    let value_col = Arc::new(Int64Array::from(vec![10, 20, 30])) as Arc<dyn arrow_array::Array>;

    // 3️⃣ Build RecordBatch
    let batch = RecordBatch::try_new(schema.clone(), vec![id_col, name_col, value_col])?;

    // 4️⃣ Extract column names
    let col_names: Vec<String> = batch
        .schema()
        .fields()
        .iter()
        .map(|f| f.name().clone())
        .collect();

    println!("Columns: {:?}", col_names);

    let mut data_map = std::collections::HashMap::new();
    for field in batch.schema().fields() {
        let col = batch.column(batch.schema().index_of(field.name())?);
        let values: Vec<Dynamic> = match col.data_type() {
            DataType::Int64 => {
                let arr = col.as_any().downcast_ref::<Int64Array>().unwrap();
                arr.iter().map(|v| Dynamic::from(v.unwrap_or(0))).collect()
            }
            DataType::Utf8 => {
                let arr = col.as_any().downcast_ref::<StringArray>().unwrap();
                arr.iter().map(|v| Dynamic::from(v.unwrap_or("").to_string())).collect()
            }
            _ => unimplemented!("Only Int64 and Utf8 supported in this test"),
        };
        data_map.insert(field.name().clone(), values);
    }

    let mut scope = Scope::new();
    for (col_name, col_vals) in data_map.iter() {
        scope.push_constant(col_name.clone(), col_vals.clone());
    }

    // 7️⃣ Rhai engine: use columns by name
    let engine = Engine::new();
    let script = r#"
        // Access columns by name
        let sum = 0;
        for v in value {
            sum += v;
        }
        let name_list = name;
        sum
    "#;

    let result: i64 = engine.eval_with_scope(&mut scope, script)?;
    println!("Sum of 'value' column: {}", result);

    Ok(())
}