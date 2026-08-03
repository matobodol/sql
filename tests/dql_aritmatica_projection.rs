mod dql_common;

use dql_common::setup_products_db;
use sql::{BinaryOp, Expr, SelectStmt, SqlValue};

#[test]
fn test_select_where_arithmetic_expressions() {
    let db = setup_products_db();
    let col_price = db.get_column_id("products", "price").unwrap();
    let col_stock = db.get_column_id("products", "stock").unwrap();

    // Nilai Aset = price * stock
    // WHERE (price * stock) > 5000
    let total_asset_expr = Expr::binary(Expr::col(col_price), BinaryOp::Mul, Expr::col(col_stock));

    let stmt = SelectStmt {
        projection: vec![],
        selection: Some(Expr::binary(
            total_asset_expr,
            BinaryOp::Gt,
            Expr::lit(5000),
        )),
        group_by: vec![],
        aggregates: vec![],
        order_by: vec![],
        limit: None,
        offset: 0,
    };

    let result = db.execute_select("products", stmt).unwrap();
    // Laptop Gaming (1500 * 10 = 15000) memenuhi syarat
    assert_eq!(result.rows.len(), 1);
    assert_eq!(
        result.rows[0].values()[1],
        SqlValue::Text("Laptop Gaming".into())
    );
}

#[test]
fn test_select_projection_with_expressions() {
    let db = setup_products_db();
    let col_id = db.get_column_id("products", "id").unwrap();
    let col_price = db.get_column_id("products", "price").unwrap();
    let col_stock = db.get_column_id("products", "stock").unwrap();

    // SELECT id, (price * stock) FROM products
    // Menguji ProjectionOperator yang mampu mengevaluasi Vec<Expr>
    let expr_id = Expr::col(col_id);
    let expr_total_val = Expr::binary(Expr::col(col_price), BinaryOp::Mul, Expr::col(col_stock));

    let stmt = SelectStmt {
        projection: vec![expr_id, expr_total_val],
        selection: None,
        group_by: vec![],
        aggregates: vec![],
        order_by: vec![],
        limit: Some(1), // Ambil baris pertama (Laptop Gaming: price=1500, stock=10)
        offset: 0,
    };

    let result = db.execute_select("products", stmt).unwrap();

    // Memastikan skema/row yang dikembalikan hanya berisi 2 kolom terproyeksi
    assert_eq!(result.schema.columns().len(), 2);
    assert_eq!(result.rows[0].values().len(), 2);

    // Verifikasi hasil proyeksi:
    // Kolom 0 = id (1)
    // Kolom 1 = price * stock (1500 * 10 = 15000)
    assert_eq!(result.rows[0].values()[0], SqlValue::Int(1));
    assert_eq!(result.rows[0].values()[1], SqlValue::Int(15000));
}
