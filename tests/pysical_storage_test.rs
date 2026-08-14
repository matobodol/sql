#[cfg(test)]
mod tests {

    use coredb::{DBM, DataType, DomainError};

    #[allow(warnings)]
    const USR_NAME: &str = "user_test";
    #[allow(warnings)]
    const PASSWD: &str = "user_passwd";
    const DB_NAME: &str = "db_test";
    const DB_RENAMED: &str = "db_renamed";
    const TBL_NAME: &str = "table_test";

    /// helper clear data
    fn remove_data_file() -> Result<(), DomainError> {
        let path = std::path::Path::new(".data");

        if path.exists() {
            std::fs::remove_dir_all(path)
                .map_err(|e| DomainError::catalog(format!("gagal hapus file: {e}")))?;
        }

        Ok(())
    }

    // A. point test:
    // 1 buat db baru.
    // 2 buat db baru menggunakan nama yg sudah terdaftar.
    // 3 ubah nama db
    // 4 ubah nama db ke nama yg sudah terdaftar.
    // 5 hapus database
    #[test]
    fn test_persistent_database_actions() -> Result<(), DomainError> {
        // hapus data lama jika ada.
        remove_data_file()?;

        let mut dbm = DBM::new();

        // 1. tes buat db baru
        let res = dbm.api_database_create(DB_NAME);
        debug_assert!(res.is_ok(), "create new db: harus sukses");

        // 2. tes buat db baru menggunakan nama yg sudah terpakai.
        let res = dbm.api_database_create(DB_NAME);
        debug_assert!(res.is_err(), "create new db (duplicate name): harus error");

        // 3. tes rename db
        let res = dbm.api_database_rename(DB_NAME, DB_RENAMED);
        debug_assert!(res.is_ok(), "rename db: harus sukses");

        // disini DB_NAME sudah tidak eksis.
        // yg terdaftar sekarang adalah DB_RENAMED.

        // tes buat db baru
        let res = dbm.api_database_create(DB_NAME);
        debug_assert!(res.is_ok(), "create new db: harus sukses");

        // 4. tes rename db ke nama yg sudah terdaftar (DB_RENAMED)
        let res = dbm.api_database_rename(DB_NAME, DB_RENAMED);
        debug_assert!(
            res.is_err(),
            "rename db ke nama yg sudah digunakan: harus error"
        );

        // disini DB_NAME masih eksis.
        // karena proses rename harusnya gagal di proses.

        // tes hapus db dengan yg eksis.
        let res = dbm.api_database_drop(DB_RENAMED);
        debug_assert!(res.is_ok(), "drop db: harus sukses");

        Ok(())
    }

    // B. test point
    #[test]
    fn test_presisten_table_actions() -> Result<(), DomainError> {
        remove_data_file()?;

        let mut dbm = DBM::new();
        // hapus data lama jika ada
        dbm.api_database_create(DB_NAME)?;

        let res = dbm.api_database_use(DB_NAME);
        debug_assert!(res.is_ok(), "use database: harus sukses");

        // buat table baru dengan kolom
        let raw_columns = vec![("id".to_string(), DataType::Int, vec![])];
        let res = dbm.api_table_create(TBL_NAME, raw_columns);
        debug_assert!(res.is_ok(), " create new table: harus sukses");

        // buat table kosong (tanpa kolom)
        let res = dbm.api_table_create("table_kosong", vec![]);
        debug_assert!(res.is_err(), "create table no column: harus error");

        Ok(())
    }
}
