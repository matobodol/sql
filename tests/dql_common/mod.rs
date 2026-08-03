use sql::{ColumnConstraint, Database, DdlAction, DmlAction, SqlType, SqlValue};

pub fn setup_products_db() -> Database {
    let mut db = Database::default();

    // Standard DDL: Tabel 'products'
    db.execute_ddl(DdlAction::CreateTable {
        name: "products".to_string(),
        columns: vec![
            (
                "id".to_string(),
                SqlType::Int,
                vec![ColumnConstraint::PrimaryKey],
            ),
            ("name".to_string(), SqlType::Text, vec![]),
            ("category".to_string(), SqlType::Text, vec![]),
            ("price".to_string(), SqlType::Int, vec![]),
            ("stock".to_string(), SqlType::Int, vec![]),
        ],
    })
    .unwrap();

    // Populasi Data Awal
    db.execute_dml(
        "products",
        &DmlAction::Insert {
            rows: vec![
                vec![
                    SqlValue::Int(1),
                    SqlValue::Text("Laptop Gaming".into()),
                    SqlValue::Text("Electronics".into()),
                    SqlValue::Int(1500),
                    SqlValue::Int(10),
                ],
                vec![
                    SqlValue::Int(2),
                    SqlValue::Text("Mouse Wireless".into()),
                    SqlValue::Text("Electronics".into()),
                    SqlValue::Int(25),
                    SqlValue::Int(50),
                ],
                vec![
                    SqlValue::Int(3),
                    SqlValue::Text("Keyboard Mechanical".into()),
                    SqlValue::Text("Electronics".into()),
                    SqlValue::Int(75),
                    SqlValue::Int(0),
                ],
                vec![
                    SqlValue::Int(4),
                    SqlValue::Text("Coffee Mug".into()),
                    SqlValue::Text("Household".into()),
                    SqlValue::Int(12),
                    SqlValue::Int(100),
                ],
                vec![
                    SqlValue::Int(5),
                    SqlValue::Text("Desk Lamp".into()),
                    SqlValue::Text("Household".into()),
                    SqlValue::Int(30),
                    SqlValue::Null,
                ],
            ],
        },
    )
    .unwrap();

    db
}
