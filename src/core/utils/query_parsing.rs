use sqlparser::ast::{Expr, Query, SetExpr, Statement, TableFactor, TableWithJoins};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use std::collections::HashSet;

pub fn extract_query_table_names(sql: &str) -> HashSet<String> {
    let dialect = GenericDialect {}; // or AnsiDialect, or your own dialect ...

    let statements = Parser::parse_sql(&dialect, sql).unwrap();
    let mut tables = HashSet::new();
    for statement in statements {
        match statement {
            Statement::Query(table_name) => {
                tables.insert(table_name.to_string());
                println!("{:?}", table_name);
            }
            Statement::Insert(table_name, ..) => {
                tables.insert(table_name.to_string());
            }
            Statement::Delete(table_name, ..) => {
                tables.insert(table_name.to_string());
            }
            _ => {}
        }
    }
    tables
}
