use sql::{ColumnConstraint, Database, DdlAction, SqlType};

#[test]
fn test_database_lookup_and_metadata_apis() {
    let mut db = Database::default();

    // 1. Uji `table_exists` sebelum tabel dibuat[span_1](start_span)[span_1](end_span)
    assert!(!db.table_exists("employees"));

    // Buat tabel untuk pengujian metadata
    let create_action = DdlAction::CreateTable {
        name: "employees".to_string(),
        columns: vec![
            (
                "emp_id".to_string(),
                SqlType::Int,
                vec![ColumnConstraint::PrimaryKey],
            ),
            ("emp_name".to_string(), SqlType::Text, vec![]),
        ],
    };
    db.execute_ddl(create_action).unwrap();

    // 2. Uji `table_exists` setelah tabel dibuat[span_2](start_span)[span_2](end_span)
    assert!(db.table_exists("employees"));

    // 3. Uji `get_table_id` dan Reverse Lookup `get_table_name`[span_3](start_span)[span_3](end_span)
    let table_id = db
        .get_table_id("employees")
        .expect("TableId harus ditemukan");
    let table_name_opt = db.get_table_name(table_id);
    assert_eq!(table_name_opt, Some("employees"));

    // 4. Uji `get_column_id`[span_4](start_span)[span_4](end_span)
    let col_id_opt = db.get_column_id("employees", "emp_name");
    assert!(col_id_opt.is_some());

    // 5. Uji `get_schema`[span_5](start_span)[span_5](end_span)
    let schema_result = db.get_schema("employees");
    assert!(schema_result.is_ok());
    let schema = schema_result.unwrap();
    assert_eq!(schema.columns().len(), 2);

    // 6. Uji `list_tables`[span_6](start_span)[span_6](end_span)
    let tables = db.list_tables();
    assert_eq!(tables.len(), 1);
    assert!(tables.contains(&"employees".to_string()));
}
