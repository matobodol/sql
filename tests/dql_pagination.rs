mod dql_common;

use dql_common::setup_products_db;
use sql::{SelectStmt, SqlValue};

#[test]
fn test_select_limit_and_offset() {
    let db = setup_products_db();

    // SELECT * FROM products LIMIT 2 OFFSET 1
    let stmt = SelectStmt {
        projection: vec![],
        selection: None,
        group_by: vec![],
        aggregates: vec![],
        order_by: vec![],
        limit: Some(2),
        offset: 1,
    };

    let result = db.execute_select("products", stmt).unwrap();
    assert_eq!(result.rows.len(), 2);

    // Baris pertama terlewati (offset 1), mendapatkan baris ke-2 (id: 2) & ke-3 (id: 3)
    assert_eq!(result.rows[0].values()[0], SqlValue::Int(2));
    assert_eq!(result.rows[1].values()[0], SqlValue::Int(3));
}
