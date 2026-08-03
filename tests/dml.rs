use sql::{
    AutoIncrement, ColumnConstraint, Database, DdlAction, DmlAction, DmlResult, Expr, SqlType,
    SqlValue,
};

fn setup_db_for_dml() -> Database {
    let mut db = Database::default();
    let create_action = DdlAction::CreateTable {
        name: "products".to_string(),
        columns: vec![
            (
                "id".to_string(),
                SqlType::Int,
                vec![
                    ColumnConstraint::PrimaryKey,
                    ColumnConstraint::AutoIncrement(AutoIncrement::Enabled { start: 1, step: 1 }),
                ],
            ),
            ("name".to_string(), SqlType::Text, vec![]),
            ("price".to_string(), SqlType::Int, vec![]),
        ],
    };
    db.execute_ddl(create_action).unwrap();
    db
}

#[test]
fn test_dml_insert_batch() {
    let mut db = setup_db_for_dml();

    let insert_action = DmlAction::Insert {
        rows: vec![
            vec![
                SqlValue::Null,
                SqlValue::Text("Laptop".into()),
                SqlValue::Int(1000),
            ],
            vec![
                SqlValue::Null,
                SqlValue::Text("Mouse".into()),
                SqlValue::Int(20),
            ],
        ],
    };

    let result = db.execute_dml("products", &insert_action);
    assert_eq!(result, Ok(DmlResult::Inserted(2)));

    let table = db.get_table("products").unwrap();
    assert_eq!(table.rows().len(), 2);
    // Cek AutoIncrement value id = 1 & id = 2
    assert_eq!(table.rows()[0].values()[0], SqlValue::Int(1));
    assert_eq!(table.rows()[1].values()[0], SqlValue::Int(2));
}

#[test]
fn test_dml_update() {
    let mut db = setup_db_for_dml();

    db.execute_dml(
        "products",
        &DmlAction::Insert {
            rows: vec![vec![
                SqlValue::Null,
                SqlValue::Text("Keyboard".into()),
                SqlValue::Int(50),
            ]],
        },
    )
    .unwrap();

    let mut assignments = std::collections::HashMap::new();
    let col_price_id = db.get_column_id("products", "price").unwrap();
    assignments.insert(col_price_id, Expr::Literal(SqlValue::Int(45)));

    let update_action = DmlAction::Update {
        assignments,
        predicate: None, // Update seluruh baris
    };

    let result = db.execute_dml("products", &update_action);
    assert_eq!(result, Ok(DmlResult::Updated(1)));

    let table = db.get_table("products").unwrap();
    assert_eq!(table.rows()[0].values()[2], SqlValue::Int(45));
}

#[test]
fn test_dml_delete() {
    let mut db = setup_db_for_dml();

    db.execute_dml(
        "products",
        &DmlAction::Insert {
            rows: vec![
                vec![
                    SqlValue::Null,
                    SqlValue::Text("Item A".into()),
                    SqlValue::Int(10),
                ],
                vec![
                    SqlValue::Null,
                    SqlValue::Text("Item B".into()),
                    SqlValue::Int(20),
                ],
            ],
        },
    )
    .unwrap();

    let delete_action = DmlAction::Delete { predicate: None };
    let result = db.execute_dml("products", &delete_action);

    assert_eq!(result, Ok(DmlResult::Deleted(2)));
    assert_eq!(db.get_table("products").unwrap().rows().len(), 0);
}
