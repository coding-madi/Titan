use std::sync::Arc;
use arrow_schema::{DataType, Schema};
use iceberg::spec::{NestedField, PrimitiveType, Schema as IcebergSchema, Type};

pub fn convert_arrow_to_iceberg_schema(arrow_schema: &Arc<Schema>) -> IcebergSchema {
    let mut field_id_counter = 0; // Iceberg requires unique field IDs

    let fields: Vec<Arc<NestedField>> = arrow_schema.fields().iter().map(|field| {
        field_id_counter += 1; // Increment for each new field
        let field_id = field_id_counter;

        let field_type = match field.data_type() {
            DataType::Boolean => Type::Primitive(PrimitiveType::Boolean),
            DataType::Int8 | DataType::Int16 | DataType::Int32 => Type::Primitive(PrimitiveType::Int),
            DataType::Int64 => Type::Primitive(PrimitiveType::Long),
            DataType::Float32 => Type::Primitive(PrimitiveType::Float),
            DataType::Float64 => Type::Primitive(PrimitiveType::Double),
            DataType::Timestamp(_, _) => Type::Primitive(PrimitiveType::Timestamp), // Assuming microsecond precision is handled by Iceberg
            DataType::Date32 | DataType::Date64 => Type::Primitive(PrimitiveType::Date),
            DataType::Binary | DataType::LargeBinary => Type::Primitive(PrimitiveType::Binary),
            DataType::Utf8 | DataType::LargeUtf8 => Type::Primitive(PrimitiveType::String),
            // Corrected FixedSizeBinary to use i32 for size
            DataType::FixedSizeBinary(size) => Type::Primitive(PrimitiveType::Fixed(*size as u64)),
            DataType::Decimal128(precision, scale) => Type::Primitive(PrimitiveType::Decimal {
                precision: 0,
                scale: 0,
            }),
            // DataType::List(list_field) => {
            //     // For list, we need to recursively convert the inner field
            //     field_id_counter += 1; // Increment for the element field
            //     let element_field_id = field_id_counter;
            //     Type::List(
            //         element_field_id,
            //         Box::new(convert_arrow_data_type_to_iceberg_type(list_field.data_type(), &mut field_id_counter)),
            //         !list_field.is_nullable(), // If Arrow list element is not nullable, Iceberg list element is required
            //     )
            // },
            // DataType::Struct(struct_fields) => {
            //     // For struct, we need to recursively convert its fields
            //     let struct_nested_fields: Vec<NestedField> = struct_fields.iter().map(|sf| {
            //         field_id_counter += 1; // Increment for each struct sub-field
            //         let sub_field_id = field_id_counter;
            //         NestedField::new(
            //             sub_field_id,
            //             &sf.name(),
            //             convert_arrow_data_type_to_iceberg_type(sf.data_type(), &mut field_id_counter),
            //             !sf.is_nullable(), // If Arrow struct field is not nullable, Iceberg struct field is required
            //         )
            //     }).collect();
            //     Type::Struct(struct_nested_fields)
            // },
            // Add more type mappings as needed
            _ => panic!("Unsupported Arrow DataType for Iceberg conversion: {:?}", field.data_type()),
        };

        // Wrap NestedField in Arc::new()
        Arc::new(NestedField::new(field_id, &field.name(), field_type, !field.is_nullable()))
    }).collect();

    // Iceberg schemas typically have a schema ID and a highest field ID.
    // For a new schema, we can assign a default schema ID (e.g., 0 or 1)
    // and the highest field ID will be our counter's final value.
    IcebergSchema::builder()
        .with_schema_id(1)
        .with_fields(fields)
        .build().unwrap()
    // IcebergSchema::new(1, fields, Some(field_id_counter))
}

// Helper function for recursive type conversion
fn convert_arrow_data_type_to_iceberg_type(arrow_data_type: &DataType, field_id_counter: &mut i32) -> Type {
    match arrow_data_type {
        DataType::Boolean => Type::Primitive(PrimitiveType::Boolean),
        DataType::Int8 | DataType::Int16 | DataType::Int32 => Type::Primitive(PrimitiveType::Int),
        DataType::Int64 => Type::Primitive(PrimitiveType::Long),
        DataType::Float32 => Type::Primitive(PrimitiveType::Float),
        DataType::Float64 => Type::Primitive(PrimitiveType::Double),
        DataType::Timestamp(_, _) => Type::Primitive(PrimitiveType::Timestamp),
        DataType::Date32 | DataType::Date64 => Type::Primitive(PrimitiveType::Date),
        DataType::Binary | DataType::LargeBinary => Type::Primitive(PrimitiveType::Binary),
        DataType::Utf8 | DataType::LargeUtf8 => Type::Primitive(PrimitiveType::String),
        DataType::FixedSizeBinary(size) => Type::Primitive(PrimitiveType::Fixed(*size as u64)),
        DataType::Decimal128(precision, scale) => Type::Primitive(PrimitiveType::Decimal {
            precision: 0,
            scale: 0,
        }),
        _ => {
            panic!("Unsupported Arrow DataType for Iceberg conversion: {:?}", arrow_data_type);
        }
    }
}