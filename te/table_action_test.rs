#[cfg(test)]
mod tests {
    use coredb::{CommandAction, DataType, Database, command::TableAction};

    /// Fungsi helper untuk inisialisasi database kosong[span_2](start_span)[span_2](end_span)
    fn setup_empty_db() -> Database {
        Database::new()
    }

    /// Fungsi helper untuk inisialisasi database yang sudah memiliki tabel "users[span_3](start_span)[span_4](start_span)"[span_3](end_span)[span_4](end_span)
    fn setup_db_with_users_table() -> Database {
        let mut db = Database::new();
        let create_action = CommandAction::TableAction {
            actions: vec![TableAction::CreateTable {
                table_name: "users".to_string(),
                columns: vec![
                    ("id".to_string(), DataType::Int, vec![]),
                    ("name".to_string(), DataType::Text, vec![]),
                ],
            }],
        };
        Database::execute(&mut db, create_action).unwrap();
        db
    }

    #[test]
    fn test_create_database() {
        let db = setup_empty_db();
        assert!(db.catalog().list_tables().is_empty());
    }

    #[test]
    fn test_create_table() {
        let mut db = setup_empty_db();

        let create_action = CommandAction::TableAction {
            actions: vec![TableAction::CreateTable {
                table_name: "users".to_string(),
                columns: vec![
                    ("id".to_string(), DataType::Int, vec![]),
                    ("name".to_string(), DataType::Text, vec![]),
                ],
            }],
        };

        // let result = Database::execute(&mut db, "", create_action);
        let result = Database::execute(&mut db, create_action);
        assert!(result.is_ok());
        assert!(db.catalog().get_table_id("users").is_some());
    }

    #[test]
    fn test_rename_table() {
        // Menggunakan helper untuk langsung mendapatkan database berisi tabel "users"
        let mut db = setup_db_with_users_table();

        let rename_action = CommandAction::TableAction {
            actions: vec![TableAction::RenameTable {
                old_table_name: "users".to_string(),
                new_table_name: "customers".to_string(),
            }],
        };

        let rename_result = Database::execute(&mut db, rename_action);
        assert!(rename_result.is_ok());
        assert!(db.catalog().get_table_id("users").is_none());
        assert!(db.catalog().get_table_id("customers").is_some());
    }

    #[test]
    fn test_drop_table() {
        // Menggunakan helper untuk langsung mendapatkan database berisi tabel "users"
        let mut db = setup_db_with_users_table();

        let drop_action = CommandAction::TableAction {
            actions: vec![TableAction::DropTable {
                table_name: "users".to_string(),
            }],
        };

        let drop_result = Database::execute(&mut db, drop_action);
        assert!(drop_result.is_ok());
        assert!(db.catalog().get_table_id("users").is_none());
    }
}
