use sql::{AlterTableAction, ColumnConstraint, ColumnPosition, Database, DdlAction, SqlType};

fn setup_db_with_users_table() -> Database {
    let mut db = Database::default();
    let create_action = DdlAction::CreateTable {
        name: "users".to_string(),
        columns: vec![
            (
                "id".to_string(),
                SqlType::Int,
                vec![ColumnConstraint::PrimaryKey],
            ),
            ("name".to_string(), SqlType::Text, vec![]),
        ],
    };
    db.execute_ddl(create_action).unwrap();
    db
}

#[test]
fn test_create_and_drop_table() {
    let mut db = Database::default();
    assert!(!db.table_exists("users"));

    // Create table
    let create_action = DdlAction::CreateTable {
        name: "users".to_string(),
        columns: vec![("id".to_string(), SqlType::Int, vec![])],
    };
    assert!(db.execute_ddl(create_action).is_ok());
    assert!(db.table_exists("users"));

    // Drop table
    let drop_action = DdlAction::DropTable {
        name: "users".to_string(),
    };
    assert!(db.execute_ddl(drop_action).is_ok());
    assert!(!db.table_exists("users"));
}

#[test]
fn test_alter_add_and_drop_column() {
    let mut db = setup_db_with_users_table();

    // Add Column
    let add_col = AlterTableAction::AddColumn {
        name: "email".to_string(),
        sql_type: SqlType::Text,
        constraints: vec![],
        position: ColumnPosition::Default,
    };
    assert!(db.execute_alter("users", vec![add_col]).is_ok());
    assert!(db.get_column_id("users", "email").is_some());

    // Drop Column
    let drop_col = AlterTableAction::DropColumn {
        name: "email".to_string(),
    };
    assert!(db.execute_alter("users", vec![drop_col]).is_ok());
    assert!(db.get_column_id("users", "email").is_none());
}

#[test]
fn test_alter_rename_table_and_column() {
    let mut db = setup_db_with_users_table();

    // Rename Column
    let rename_col = AlterTableAction::RenameColumn {
        old_name: "name".to_string(),
        new_name: "full_name".to_string(),
    };
    assert!(db.execute_alter("users", vec![rename_col]).is_ok());
    assert!(db.get_column_id("users", "full_name").is_some());

    // Rename Table
    let rename_table = AlterTableAction::RenameTable {
        new_name: "accounts".to_string(),
    };
    assert!(db.execute_alter("users", vec![rename_table]).is_ok());
    assert!(!db.table_exists("users"));
    assert!(db.table_exists("accounts"));
}

#[test]
fn test_alter_rollback_on_failure() {
    let mut db = setup_db_with_users_table();

    // Rangkaian transaksi ALTER: satu valid, satu gagal (menghapus kolom yang tidak ada)
    let actions = vec![
        AlterTableAction::AddColumn {
            name: "age".to_string(),
            sql_type: SqlType::Int,
            constraints: vec![],
            position: ColumnPosition::Default,
        },
        AlterTableAction::DropColumn {
            name: "non_existent_col".to_string(),
        },
    ];

    // Eksekusi harus gagal dan membatalkan penambahan kolom "age"
    assert!(db.execute_alter("users", actions).is_err());
    assert!(db.get_column_id("users", "age").is_none());
}
