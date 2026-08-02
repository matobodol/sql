use std::collections::HashMap;

use sql::catalog::db_function::dml_action::{DmlAction, DmlResult};
use sql::catalog::table::Table;
use sql::domain::expr::{BinaryOp, Expr};
use sql::domain::id::{ColumnId, TableId};
use sql::domain::schema::ColumnDef;
use sql::domain::{ColumnConstraint, Schema, SqlType, SqlValue};

fn setup_users_table() -> Table {
    let cols = vec![
        ColumnDef::with_constraints(
            ColumnId(1),
            "id",
            SqlType::Int,
            vec![ColumnConstraint::PrimaryKey],
        ),
        ColumnDef::with_constraints(
            ColumnId(2),
            "username",
            SqlType::Text,
            vec![ColumnConstraint::Unique],
        ),
        ColumnDef::with_constraints(
            ColumnId(3),
            "age",
            SqlType::Int,
            vec![ColumnConstraint::NotNull],
        ),
    ];

    let schema = Schema::new(cols).expect("Skema tabel harus valid");
    Table::new(TableId(1), "users", schema)
}

#[test]
fn test_multiple_insert_and_atomic_rollback() {
    let mut table = setup_users_table();

    // 1. Multiple Insert Valid (2 baris sekaligus)
    let batch1 = vec![
        vec![
            SqlValue::Int(1),
            SqlValue::Text("alice".into()),
            SqlValue::Int(25),
        ],
        vec![
            SqlValue::Int(2),
            SqlValue::Text("bob".into()),
            SqlValue::Int(30),
        ],
    ];
    let res1 = table.execute_dml(DmlAction::Insert { rows: batch1 });
    assert_eq!(res1.unwrap(), DmlResult::Inserted(2));
    assert_eq!(table.rows().len(), 2);

    // 2. Multiple Insert dengan Error pada Baris ke-2 (Atomic Rollback Test)
    // Row 1: Valid (id = 3, username = "charlie")
    // Row 2: Invalid! (id = 1 -> Duplikat Primary Key dari Alice)
    let batch_with_error = vec![
        vec![
            SqlValue::Int(3),
            SqlValue::Text("charlie".into()),
            SqlValue::Int(22),
        ],
        vec![
            SqlValue::Int(1),
            SqlValue::Text("david".into()),
            SqlValue::Int(28),
        ],
    ];
    let res2 = table.execute_dml(DmlAction::Insert {
        rows: batch_with_error,
    });

    // Harus melempar error
    assert!(res2.is_err());

    // GARANSI ALL-OR-NOTHING:
    // Charlie (Row 1 yang valid) TIDAK BOLEH tersimpan di DB karena batch gagal di baris ke-2
    assert_eq!(table.rows().len(), 2);
}

#[test]
fn test_update_with_predicate() {
    let mut table = setup_users_table();

    table
        .insert_batch(vec![
            vec![
                SqlValue::Int(1),
                SqlValue::Text("alice".into()),
                SqlValue::Int(20),
            ],
            vec![
                SqlValue::Int(2),
                SqlValue::Text("bob".into()),
                SqlValue::Int(30),
            ],
        ])
        .unwrap();

    let mut assignments = HashMap::new();
    assignments.insert(ColumnId(3), Expr::lit(SqlValue::Int(21)));

    let predicate = Expr::binary(
        Expr::col(ColumnId(1)),
        BinaryOp::Eq,
        Expr::lit(SqlValue::Int(1)),
    );

    let result = table.execute_dml(DmlAction::Update {
        assignments,
        predicate: Some(predicate),
    });

    assert_eq!(result.unwrap(), DmlResult::Updated(1));
    assert_eq!(table.rows()[0].values()[2], SqlValue::Int(21));
    assert_eq!(table.rows()[1].values()[2], SqlValue::Int(30));
}

#[test]
fn test_delete_targeted_rows() {
    let mut table = setup_users_table();

    table
        .insert_batch(vec![
            vec![
                SqlValue::Int(1),
                SqlValue::Text("alice".into()),
                SqlValue::Int(20),
            ],
            vec![
                SqlValue::Int(2),
                SqlValue::Text("bob".into()),
                SqlValue::Int(30),
            ],
        ])
        .unwrap();

    let predicate = Expr::binary(
        Expr::col(ColumnId(2)),
        BinaryOp::Eq,
        Expr::lit(SqlValue::Text("alice".into())),
    );

    let result = table.execute_dml(DmlAction::Delete {
        predicate: Some(predicate),
    });

    assert_eq!(result.unwrap(), DmlResult::Deleted(1));
    assert_eq!(table.rows().len(), 1);
    assert_eq!(table.rows()[0].values()[1], SqlValue::Text("bob".into()));
}
