mod dql_common;

use dql_common::setup_products_db;
use sql::{BinaryOp, Expr, SelectStmt, SqlValue};

#[test]
fn test_select_where_and_or_logic() {
    let db = setup_products_db();
    let col_cat = db.get_column_id("products", "category").unwrap();
    let col_price = db.get_column_id("products", "price").unwrap();

    // WHERE category = 'Electronics' AND price < 50
    let where_clause = Expr::binary(
        Expr::binary(Expr::col(col_cat), BinaryOp::Eq, Expr::lit("Electronics")),
        BinaryOp::And,
        Expr::binary(Expr::col(col_price), BinaryOp::Lt, Expr::lit(50)),
    );

    let stmt = SelectStmt {
        projection: vec![],
        selection: Some(where_clause),
        group_by: vec![],
        aggregates: vec![],
        order_by: vec![],
        limit: None,
        offset: 0,
    };

    let result = db.execute_select("products", stmt).unwrap();
    assert_eq!(result.rows.len(), 1); // Hanya 'Mouse Wireless'
}

#[test]
fn test_select_where_like_pattern() {
    let db = setup_products_db();
    let col_name = db.get_column_id("products", "name").unwrap();

    // WHERE name LIKE '%less' (Mencari teks berakhiran "less")
    let stmt = SelectStmt {
        projection: vec![],
        selection: Some(Expr::binary(
            Expr::col(col_name),
            BinaryOp::Like,
            Expr::lit("%less"),
        )),
        group_by: vec![],
        aggregates: vec![],
        order_by: vec![],
        limit: None,
        offset: 0,
    };

    let result = db.execute_select("products", stmt).unwrap();
    assert_eq!(result.rows.len(), 1);
    assert_eq!(
        result.rows[0].values()[1],
        SqlValue::Text("Mouse Wireless".into())
    );
}

#[test]
fn test_select_where_in_list() {
    let db = setup_products_db();
    let col_id = db.get_column_id("products", "id").unwrap();

    // WHERE id IN (1, 3, 5)
    let stmt = SelectStmt {
        projection: vec![],
        selection: Some(Expr::in_list(
            Expr::col(col_id),
            vec![Expr::lit(1), Expr::lit(3), Expr::lit(5)],
        )),
        group_by: vec![],
        aggregates: vec![],
        order_by: vec![],
        limit: None,
        offset: 0,
    };

    let result = db.execute_select("products", stmt).unwrap();
    assert_eq!(result.rows.len(), 3);
}
