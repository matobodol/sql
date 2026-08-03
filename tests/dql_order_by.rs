mod dql_common;

use dql_common::setup_products_db;
use sql::{Expr, OrderByExpr, SelectStmt, SortOrder, SqlValue};

#[test]
fn test_select_order_by_asc_desc() {
    let db = setup_products_db();
    let col_price = db.get_column_id("products", "price").unwrap();

    // 1. SELECT * FROM products ORDER BY price ASC
    let stmt_asc = SelectStmt {
        projection: vec![],
        selection: None,
        group_by: vec![],
        aggregates: vec![],
        order_by: vec![OrderByExpr {
            expr: Expr::col(col_price),
            order: SortOrder::Ascending,
        }],
        limit: None,
        offset: 0,
    };

    let res_asc = db.execute_select("products", stmt_asc).unwrap();
    // Harga terendah (12) harus di posisi pertama
    assert_eq!(res_asc.rows[0].values()[3], SqlValue::Int(12));

    // 2. SELECT * FROM products ORDER BY price DESC
    let stmt_desc = SelectStmt {
        projection: vec![],
        selection: None,
        group_by: vec![],
        aggregates: vec![],
        order_by: vec![OrderByExpr {
            expr: Expr::col(col_price),
            order: SortOrder::Descending,
        }],
        limit: None,
        offset: 0,
    };

    let res_desc = db.execute_select("products", stmt_desc).unwrap();
    // Harga tertinggi (1500) harus di posisi pertama
    assert_eq!(res_desc.rows[0].values()[3], SqlValue::Int(1500));
}

#[test]
fn test_select_order_by_nulls_behavior() {
    let db = setup_products_db();
    let col_stock = db.get_column_id("products", "stock").unwrap();

    // SELECT * FROM products ORDER BY stock ASC
    // Sesuai aturan Ord pada SqlValue, NULL dianggap paling kecil (NULLS FIRST)
    let stmt = SelectStmt {
        projection: vec![],
        selection: None,
        group_by: vec![],
        aggregates: vec![],
        order_by: vec![OrderByExpr {
            expr: Expr::col(col_stock),
            order: SortOrder::Ascending,
        }],
        limit: None,
        offset: 0,
    };

    let res = db.execute_select("products", stmt).unwrap();
    // Baris pertama (Desk Lamp) stock-nya harus Null
    assert_eq!(res.rows[0].values()[4], SqlValue::Null);
}

#[test]
fn test_select_order_by_multiple_columns() {
    let db = setup_products_db();
    let col_cat = db.get_column_id("products", "category").unwrap();
    let col_price = db.get_column_id("products", "price").unwrap();

    // SELECT * FROM products ORDER BY category ASC, price DESC
    let stmt = SelectStmt {
        projection: vec![],
        selection: None,
        group_by: vec![],
        aggregates: vec![],
        order_by: vec![
            OrderByExpr {
                expr: Expr::col(col_cat),
                order: SortOrder::Ascending,
            },
            OrderByExpr {
                expr: Expr::col(col_price),
                order: SortOrder::Descending,
            },
        ],
        limit: None,
        offset: 0,
    };

    let res = db.execute_select("products", stmt).unwrap();

    // Kategori pertama adalah "Electronics" (E < H)
    // Produk Electronics dengan harga tertinggi (Laptop Gaming: 1500) harus berada paling depan
    assert_eq!(
        res.rows[0].values()[2],
        SqlValue::Text("Electronics".into())
    );
    assert_eq!(res.rows[0].values()[3], SqlValue::Int(1500));
}
