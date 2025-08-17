use std::sync::Arc;
use arrow_array::{Int64Array, StringArray, RecordBatch};
use arrow_schema::{DataType, Field, Schema};



pub fn mock_arrow_data() -> RecordBatch {
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, false),
        Field::new("country", DataType::Utf8, false),
        Field::new("sales", DataType::Int64, false),
    ]));

    let id_col = Arc::new(Int64Array::from(vec![1, 2, 3, 4, 5])) as _;
    let name_col = Arc::new(StringArray::from(vec!["Alice", "Bob", "Carol", "Dave", "Eve"])) as _;
    let country_col = Arc::new(StringArray::from(vec!["US", "US", "IN", "IN", "US"])) as _;
    let sales_col = Arc::new(Int64Array::from(vec![100, 200, 150, 120, 300])) as _;

    let batch = RecordBatch::try_new(schema.clone(), vec![id_col, name_col, country_col, sales_col]).unwrap();
    batch
}

