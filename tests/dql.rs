use sql::{ColumnConstraint, Database, DdlAction, DmlAction, SelectStmt, Show, SqlType, SqlValue};

fn setup_db_for_dql() -> Database {
    let mut db = Database::default();

    // Table 1: users
    db.execute_ddl(DdlAction::CreateTable {
        name: "users".to_string(),
        columns: vec![
            (
                "id".to_string(),
                SqlType::Int,
                vec![ColumnConstraint::PrimaryKey],
            ),
            ("username".to_string(), SqlType::Text, vec![]),
        ],
    })
    .unwrap();

    // Table 2: orders
    db.execute_ddl(DdlAction::CreateTable {
        name: "orders".to_string(),
        columns: vec![("id".to_string(), SqlType::Int, vec![])],
    })
    .unwrap();

    db.execute_dml(
        "users",
        &DmlAction::Insert {
            rows: vec![
                vec![SqlValue::Int(1), SqlValue::Text("Alice".into())],
                vec![SqlValue::Int(2), SqlValue::Text("Bob".into())],
            ],
        },
    )
    .unwrap();

    db
}

#[test]
fn test_execute_show_tables() {
    let db = setup_db_for_dql();

    // SHOW TABLES
    let res = db.execute_show(Show::Tables).unwrap();
    assert_eq!(res.rows.len(), 2);

    // SHOW TABLES LIKE 'user%'
    let res_like = db.execute_show(Show::TablesLike("user%")).unwrap();
    assert_eq!(res_like.rows.len(), 1);
    assert_eq!(res_like.rows[0].values()[0], SqlValue::Text("users".into()));
}

#[test]
fn test_execute_select_basic() {
    let db = setup_db_for_dql();

    let stmt = SelectStmt {
        projection: vec![],
        selection: None,
        group_by: vec![],
        aggregates: vec![],
        order_by: vec![],
        limit: None,
        offset: 0,
    };

    let result = db.execute_select("users", stmt);
    assert!(result.is_ok());
    let dql_res = result.unwrap();
    assert_eq!(dql_res.rows.len(), 2);
}
