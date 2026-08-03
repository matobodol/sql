mod dql_common;

use dql_common::setup_products_db;
use sql::{BinaryOp, Expr, SelectStmt};

#[test]
fn test_select_where_equality_and_comparison() {
    let db = setup_products_db();
    let col_price = db.get_column_id("products", "price").unwrap();

    // WHERE price > 50
    let stmt = SelectStmt {
        projection: vec![],
        selection: Some(Expr::binary(
            Expr::col(col_price),
            BinaryOp::Gt,
            Expr::lit(50),
        )),
        group_by: vec![],
        aggregates: vec![],
        order_by: vec![],
        limit: None,
        offset: 0,
    };

    let result = db.execute_select("products", stmt).unwrap();
    // Berhasil mengambil 'Laptop Gaming' (1500) dan 'Keyboard Mechanical' (75)
    assert_eq!(result.rows.len(), 2);
}

#[test]
fn test_select_where_is_null_and_is_not_null() {
    let db = setup_products_db();
    let col_stock = db.get_column_id("products", "stock").unwrap();

    // 1. WHERE stock IS NULL
    let stmt_null = SelectStmt {
        projection: vec![],
        selection: Some(Expr::is_null(Expr::col(col_stock))),
        group_by: vec![],
        aggregates: vec![],
        order_by: vec![],
        limit: None,
        offset: 0,
    };

    let res_null = db.execute_select("products", stmt_null).unwrap();
    assert_eq!(res_null.rows.len(), 1); // Desk Lamp (stock NULL)

    // 2. WHERE stock IS NOT NULL
    let stmt_not_null = SelectStmt {
        projection: vec![],
        selection: Some(Expr::is_not_null(Expr::col(col_stock))),
        group_by: vec![],
        aggregates: vec![],
        order_by: vec![],
        limit: None,
        offset: 0,
    };

    let res_not_null = db.execute_select("products", stmt_not_null).unwrap();
    assert_eq!(res_not_null.rows.len(), 4);
}
