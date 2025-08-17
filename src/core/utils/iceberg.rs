use arrow_array::RecordBatch;
use arrow_schema::{DataType, Schema};
use iceberg::TableIdent;
use iceberg::spec::{
    ListType, NestedField, NestedFieldRef, PrimitiveType, Schema as IcebergSchema, StructType, Type,
};
use std::sync::Arc;

pub fn convert_arrow_to_iceberg_schema(arrow_schema: &Arc<Schema>) -> IcebergSchema {
    let mut field_id_counter = 0; // Iceberg requires unique field IDs

    let fields: Vec<Arc<NestedField>> = arrow_schema
        .fields()
        .iter()
        .map(|field| {
            field_id_counter += 1; // Increment for each new field
            let field_id = field_id_counter;

            let field_type = match field.data_type() {
                DataType::Boolean => Type::Primitive(PrimitiveType::Boolean),
                DataType::Int8 | DataType::Int16 | DataType::Int32 => {
                    Type::Primitive(PrimitiveType::Int)
                }
                DataType::Int64 => Type::Primitive(PrimitiveType::Long),
                DataType::Float32 => Type::Primitive(PrimitiveType::Float),
                DataType::Float64 => Type::Primitive(PrimitiveType::Double),
                DataType::Timestamp(_, _) => Type::Primitive(PrimitiveType::Timestamp), // Assuming microsecond precision is handled by Iceberg
                DataType::Date32 | DataType::Date64 => Type::Primitive(PrimitiveType::Date),
                DataType::Binary | DataType::LargeBinary => Type::Primitive(PrimitiveType::Binary),
                DataType::Utf8 | DataType::LargeUtf8 => Type::Primitive(PrimitiveType::String),
                // Corrected FixedSizeBinary to use i32 for size
                DataType::FixedSizeBinary(size) => {
                    Type::Primitive(PrimitiveType::Fixed(*size as u64))
                }
                DataType::Decimal128(_precision, _scale) => {
                    Type::Primitive(PrimitiveType::Decimal {
                        precision: 0,
                        scale: 0,
                    })
                }
                DataType::List(list_field) => {
                    // For list, we need to recursively convert the inner field
                    field_id_counter += 1;
                    let element_field_id = &mut field_id_counter;
                    let element_type = convert_arrow_data_type_to_iceberg_type(
                        list_field.data_type(),
                        element_field_id,
                    );
                    Type::List(ListType::new(NestedFieldRef::new(NestedField::new(
                        field_id_counter,
                        list_field.name(),
                        element_type,
                        !list_field.is_nullable(),
                    ))))
                }
                DataType::Struct(fields) => {
                    let struct_fields: Vec<NestedFieldRef> = fields
                        .iter()
                        .map(|sub_field| {
                            field_id_counter += 1;
                            let mut_clone = &mut field_id_counter.clone();
                            let iceberg_type = convert_arrow_data_type_to_iceberg_type(
                                sub_field.data_type(),
                                mut_clone,
                            );
                            NestedFieldRef::new(NestedField::new(
                                field_id_counter,
                                sub_field.name(),
                                iceberg_type,
                                !sub_field.is_nullable(),
                            ))
                        })
                        .collect();
                    Type::Struct(StructType::new(struct_fields))
                }
                // Add more type mappings as needed
                _ => panic!(
                    "Unsupported Arrow DataType for Iceberg conversion: {:?}",
                    field.data_type()
                ),
            };

            // Wrap NestedField in Arc::new()
            Arc::new(NestedField::new(
                field_id,
                field.name(),
                field_type,
                !field.is_nullable(),
            ))
        })
        .collect();

    // Iceberg schemas typically have a schema ID and a highest field ID.
    // For a new schema, we can assign a default schema ID (e.g., 0 or 1)
    // and the highest field ID will be our counter's final value.
    IcebergSchema::builder()
        .with_schema_id(1)
        .with_fields(fields)
        .build()
        .unwrap()
    // IcebergSchema::new(1, fields, Some(field_id_counter))
}

// Helper function for recursive type conversion
fn convert_arrow_data_type_to_iceberg_type(
    arrow_data_type: &DataType,
    field_id_counter: &mut i32,
) -> Type {
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
        DataType::Decimal128(_precision, _scale) => Type::Primitive(PrimitiveType::Decimal {
            precision: 0,
            scale: 0,
        }),
        DataType::List(list_field) => {
            *field_id_counter += 1; // Increment for the list element field
            let element_field_id = *field_id_counter; // Store the ID for the element field

            let element_type = convert_arrow_data_type_to_iceberg_type(
                list_field.data_type(),
                field_id_counter, // Pass mutable reference
            );
            Type::List(ListType::new(NestedFieldRef::new(NestedField::new(
                element_field_id, // Use the ID generated for the element field
                list_field.name(),
                element_type,
                !list_field.is_nullable(),
            ))))
        }
        DataType::Struct(fields) => {
            let struct_fields: Vec<NestedFieldRef> = fields
                .iter()
                .map(|sub_field| {
                    *field_id_counter += 1; // Increment for each sub-field
                    let sub_field_id = *field_id_counter; // Store the ID for the sub-field

                    let iceberg_type = convert_arrow_data_type_to_iceberg_type(
                        sub_field.data_type(),
                        field_id_counter, // Pass the mutable reference down
                    );
                    NestedFieldRef::new(NestedField::new(
                        sub_field_id, // Use the ID generated for this sub-field
                        sub_field.name(),
                        iceberg_type,
                        !sub_field.is_nullable(),
                    ))
                })
                .collect();
            Type::Struct(StructType::new(struct_fields))
        }
        _ => {
            panic!("Unsupported Arrow DataType for Iceberg conversion: {arrow_data_type:?}");
        }
    }
}

pub fn make_table_ident(table: String) -> Result<TableIdent, String> {
    TableIdent::from_strs(vec!["log", &table.clone()])
        .map_err(|e| format!("Failed to create TableIdent: {:?}", e))
}

pub fn print_schema_info(batch: &RecordBatch) {
    println!("Merged RecordBatch Schema (Arrow): {:?}", batch.schema());
    for (i, field) in batch.schema().fields().iter().enumerate() {
        println!(
            "Arrow Field: Index={}, Name='{}', DataType={:?}",
            i,
            field.name(),
            field.data_type()
        );
    }
}
