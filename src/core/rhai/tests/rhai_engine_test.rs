
#[cfg(test)]
pub mod test {
    use crate::application::actors::broadcast_actor::{Metadata, RecordBatchWrapper};
    use crate::core::rhai::query_planner::QueryPlanner;
    use crate::core::rhai::rhai_engine::{Record, RhaiEngine, execution_engine};
    use arrow_array::builder::{BooleanBuilder, StringBuilder};
    use arrow_array::{ArrayRef, BooleanArray, Int8Array, Int64Array, RecordBatch, StringArray};
    use arrow_schema::{DataType, Field, Schema};
    use rand::Rng;
    use rand::distributions::Alphanumeric;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Instant;
    use crate::core::rhai::rhai_executor;
    use crate::core::rhai::rhai_executor::RhaiExecutor;

    fn generate_data(n: usize) -> Vec<Record> {
        let mut dataset = Vec::new();
        for i in 0..1_000 {
            let mut rec = HashMap::new();
            rec.insert("service_id".to_string(), i % 100);
            rec.insert("cpu_usage".to_string(), (i % 200) as i64);
            dataset.push(rec);
        }
        dataset
    }

    fn create_schema() -> Schema {
        let schema = Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, false),
            Field::new("active", DataType::Boolean, false),
            Field::new("binary", DataType::Int64, false),
        ]);
        schema
    }

    fn create_dummy_dataset(n_rows: usize) -> RecordBatch {
        let schema = Arc::new(create_schema());

        // ids = 1..=n_rows
        let ids: Int64Array = (1..=n_rows as i64).collect();

        // names = random 5-char strings
        let mut rng = rand::thread_rng();
        let mut name_builder = StringBuilder::new();
        for _ in 0..n_rows {
            let s: String = (0..5).map(|_| rng.sample(Alphanumeric) as char).collect();
            name_builder.append_value(&s);
        }
        let names: StringArray = name_builder.finish();

        // active = alternating true/false
        let mut active_builder = BooleanBuilder::new();
        for i in 0..n_rows {
            active_builder.append_value(i % 2 == 0);
        }
        let active: BooleanArray = active_builder.finish();

        let binary: Int64Array = (1..=n_rows as i64).map(|num| num % 2).collect();

        let columns: Vec<ArrayRef> = vec![
            Arc::new(ids),
            Arc::new(names),
            Arc::new(active),
            Arc::new(binary),
        ];

        RecordBatch::try_new(schema, columns).unwrap()
    }

    /// Sql:
    /// select Sum(id) from flight where (id > 88 and id < 149) or id = 100 or name like 'Ra%' group_by binary;
    #[test]
    fn group_by_aggregates() {
        let script = r#"
            let cond = or([
                and([ gt_int("id", 88), lt_int("id", 149) ]),
                eq_int("id", 100),
                like("name", "Ra%")
            ]);

            let q = query();
            q = q.filter(cond);
            q = q.group_by(["binary"]);
            q = q.agg("Sum", "id");
            q = q.limit(100);
            q;
            "#;

        let engine = execution_engine();
        let mut scope = rhai::Scope::new();
        let plan: QueryPlanner = engine.eval_with_scope(&mut scope, script).unwrap();
        let dataset = create_dummy_dataset(1_000);
        let record_batch_wrapper = RecordBatchWrapper::new(
            Metadata::new("flight", 1, Arc::new(create_schema())),
            &dataset,
        );
        let start_time = Instant::now();

        // Actual tests
        let rhai_executor = RhaiExecutor::new("test_flight".to_string());

        // Actual tests
        let filtered = RhaiExecutor::apply(record_batch_wrapper, plan);

        assert!(filtered.is_ok());
        let elapsed_time = start_time.elapsed();
        println!("Time taken {:?}", elapsed_time);
        println!("{:?}", filtered);
        assert!(filtered.is_ok())
    }

    /// Sql:
    /// select Sum(id) from flight where (id > 88 and id < 149) or id = 100 or name like 'Ra%';
    #[test]
    fn sum_all_without_any_group() {
        let script = r#"
            let cond = or([
                and([ gt_int("id", 88), lt_int("id", 149) ]),
                eq_int("id", 100),
                like("name", "R%")
            ]);

            let q = query();
            q = q.filter(cond);
            q = q.agg("Sum", "id");
            q = q.limit(100);
            q;
            "#;

        let engine = execution_engine();
        let mut scope = rhai::Scope::new();
        let plan: QueryPlanner = engine.eval_with_scope(&mut scope, script).unwrap();
        let dataset = create_dummy_dataset(1000_000);
        let record_batch_wrapper = RecordBatchWrapper::new(
            Metadata::new("flight", 1, Arc::new(create_schema())),
            &dataset,
        );
        let start_time = Instant::now();

        let rhai_executor = RhaiExecutor::new("test_flight".to_string());

        // Actual tests
        let filtered = RhaiExecutor::apply(record_batch_wrapper, plan);

        assert!(filtered.is_ok());
        let elapsed_time = start_time.elapsed();
        println!("Time taken {:?}", elapsed_time);
        assert!(
            filtered.is_ok(),
            "Expected Ok, got {:?}",
            filtered.unwrap_err()
        )
    }

    /// select Sum(id) from flight group by id where (id > 88 and id < 149) or id = 100 or name like 'Ra%';
    #[test]
    fn fail_group_by_binary_and_sum_by_id() {
        let script = r#"
            let cond = or([
                and([ gt_int("id", 88), lt_int("id", 149) ]),
                eq_int("id", 100),
                like("name", "R%")
            ]);

            let q = query();
            q = q.filter(cond);
            q = q.group_by(["name"]);
            q = q.agg("Sum", "id");
            q = q.limit(100);
            q;
            "#;

        let engine = execution_engine();
        let mut scope = rhai::Scope::new();
        let plan: QueryPlanner = engine.eval_with_scope(&mut scope, script).unwrap();
        let dataset = create_dummy_dataset(1_000);
        println!("{:?}", dataset);
        let record_batch_wrapper = RecordBatchWrapper::new(
            Metadata::new("flight", 1, Arc::new(create_schema())),
            &dataset,
        );
        let start_time = Instant::now();

        // Actual tests
        let rhai_executor = RhaiExecutor::new("test_flight".to_string());

        // Actual tests
        let filtered = RhaiExecutor::apply(record_batch_wrapper, plan);

        assert!(filtered.is_ok());
        let elapsed_time = start_time.elapsed();
        println!("Time taken {:?}", elapsed_time);
        println!("{:?}", filtered);
    }
}
