#[cfg(test)]
mod tests {
    use coredb::DatabaseManager;

    #[test]
    fn test_database_manager_end_to_end_workflow() {
        // Inisialisasi DatabaseManager (default user: root)[span_3](start_span)[span_3](end_span)
        let mut db_mgr = DatabaseManager::new();

        // 1. Pengujian Pembuatan Database
        let db_name = "shop_db";
        assert!(db_mgr.create_database(db_name).is_ok());

        // 2. Pengujian Duplikasi Database (Harus Error)
        assert!(db_mgr.create_database(db_name).is_err());

        // 3. Pengujian Menggunakan Database Aktif
        assert!(db_mgr.use_database(db_name).is_ok());
        assert_eq!(db_mgr.active_db_name(), Some(db_name));

        // 4. Pengujian Eksekusi Perintah melalui Facade (`execute`)
        // Contoh jika Anda ingin menguji perintah DQL/DDL lewat command action:
        /*
        let action = CommandAction::Select {
            table_name: "users".to_string(),
            statements: SelectStmt { ... },
        };

        let result = db_mgr.execute(action);
        assert!(result.is_ok());
        */
    }

    #[test]
    fn test_user_authorization_and_isolation() {
        let mut db_mgr = DatabaseManager::new();

        // Root membuat database
        db_mgr.create_database("admin_db").unwrap();

        // Membuat user baru (memerlukan hak akses admin)[span_4](start_span)[span_4](end_span)
        let create_user_res = db_mgr.create_user("alice", "secure_password");
        assert!(create_user_res.is_ok());

        // Login sebagai alice
        assert!(db_mgr.login("alice", "secure_password").is_ok());

        // Alice mencoba membuat database sendiri
        assert!(db_mgr.create_database("alice_db").is_ok());

        // Alice seharusnya tidak bisa mengakses database milik root jika terisolasi
        assert!(db_mgr.use_database("admin_db").is_err());
    }
}
