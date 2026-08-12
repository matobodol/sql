#[cfg(test)]
mod tests {

    use coredb::{
        ColumnConstraint, CommandAction, DataType, DatabaseManager, DomainError, TableAction,
    };

    #[test]
    fn test_persistent_file_layout() -> Result<(), DomainError> {
        // 1. Inisialisasi DatabaseManager (secara otomatis membuat folder data/root)
        let mut db_manager = DatabaseManager::new();

        // 2. Simpan user manager ke disk untuk menghasilkan file global_users.bin
        // db_manager.save_users()?;
        let global_user_path = std::path::Path::new("data/global_users.bin");
        assert!(
            global_user_path.exists(),
            "File global_users.bin harus tercipta di disk"
        );

        // 3. Buat database baru bernama "test_db" di bawah user aktif ('root')
        db_manager.create_database("test_db")?;
        db_manager.use_database("test_db")?;

        // 4. Ambil referensi database aktif dan buat tabel fisik (misal: "karyawan")
        let db = db_manager.active_database_mut()?;
        db.execute(CommandAction::TableAction {
            actions: vec![TableAction::CreateTable {
                table_name: "karyawan".to_string(),
                columns: vec![(
                    "id".to_string(),
                    DataType::Int,
                    vec![ColumnConstraint::PrimaryKey],
                )],
            }],
        })?;

        // 5. Panggil save_to_disk untuk menulis metadata.bin dan flush file .db
        // db.save_to_disk()?;

        // 6. Verifikasi keberadaan seluruh file pada struktur layout persisten
        let metadata_path = std::path::Path::new("data/root/test_db/metadata.bin");
        let table_path = std::path::Path::new("data/root/test_db/karyawan.db");

        assert!(
            global_user_path.exists(),
            "global_users.bin tidak ditemukan"
        );
        assert!(
            metadata_path.exists(),
            "metadata.bin tidak ditemukan di dalam folder database"
        );
        assert!(
            table_path.exists(),
            "karyawan.db tidak ditemukan di dalam folder database"
        );

        Ok(())
    }
}
