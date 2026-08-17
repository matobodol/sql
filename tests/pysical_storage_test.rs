#[cfg(test)]
mod tests {
    use sql::{CMD, ColumnConstraint, ColumnPosition, DBM, DataType, DomainError, Increment};

    #[allow(warnings)]
    const USR_NAME: &str = "user_test";
    #[allow(warnings)]
    const PASSWD: &str = "user_passwd";
    const DB_NAME: &str = "db_test";
    const DB_RENAMED: &str = "db_renamed";
    const TBL_NAME: &str = "table_test";
    const TBL_RENAMED: &str = "table_renamed";

    /// helper clear data
    fn remove_data_file() -> Result<(), DomainError> {
        let path = std::path::Path::new(".data");

        if path.exists() {
            std::fs::remove_dir_all(path)
                .map_err(|e| DomainError::metadata(format!("gagal hapus file: {e}")))?;
        }

        Ok(())
    }

    // A. point test:
    // 1 buat db baru (uniq name).
    // 2 buat db baru (non uniq name).
    // 3 ubah nama db (uniq name)
    // 4 ubah nama db ke nama yg sudah terdaftar (non uniq name).
    // 5 hapus database
    #[test]
    fn test_persistent_database_actions() -> Result<(), DomainError> {
        // hapus data lama jika ada.
        remove_data_file()?;

        let mut dbm = DBM::new();

        // 1. tes buat db baru
        let res = dbm.execute(vec![CMD::CreateDatabase {
            db_name: DB_NAME.into(),
        }]);
        debug_assert!(res.is_ok(), "create new db: harus sukses");

        // 2. tes buat db baru menggunakan nama yg sudah terpakai.
        let res = dbm.execute(vec![CMD::CreateDatabase {
            db_name: DB_NAME.to_string(),
        }]);
        debug_assert!(res.is_err(), "create new db (duplicate name): harus error");

        // 3. tes rename db
        let res = dbm.execute(vec![CMD::RenamDatabase {
            old_name: DB_NAME.to_string(),
            new_name: DB_RENAMED.to_string(),
        }]);
        debug_assert!(res.is_ok(), "rename db: harus sukses");

        // disini DB_NAME sudah tidak eksis.
        // yg terdaftar sekarang adalah DB_RENAMED.

        // tes buat db baru
        let res = dbm.execute(vec![CMD::CreateDatabase {
            db_name: DB_NAME.to_string(),
        }]);
        debug_assert!(res.is_ok(), "create new db: harus sukses");

        // 4. tes rename db ke nama yg sudah terdaftar (DB_RENAMED)
        let res = dbm.execute(vec![CMD::RenamDatabase {
            old_name: DB_NAME.to_string(),
            new_name: DB_RENAMED.to_string(),
        }]);
        debug_assert!(
            res.is_err(),
            "rename db ke nama yg sudah digunakan: harus error"
        );

        // disini DB_NAME masih eksis.
        // karena proses rename sebelumnya gagal diproses.

        // tes hapus db dengan yg eksis.
        let res = dbm.execute(vec![CMD::DropDatabase {
            db_name: DB_NAME.to_string(),
        }]);
        debug_assert!(res.is_ok(), "drop db: harus sukses");

        Ok(())
    }

    // B. test point
    #[test]
    fn test_presistent_table_actions() -> Result<(), DomainError> {
        remove_data_file()?;

        let mut dbm = DBM::new();
        // hapus data lama jika ada
        let res = dbm.execute(vec![CMD::CreateDatabase {
            db_name: DB_NAME.to_string(),
        }]);
        debug_assert!(res.is_ok(), "creat db: harus ok");

        let res = dbm.execute(vec![CMD::UseDatabase {
            db_name: DB_NAME.to_string(),
        }]);
        debug_assert!(res.is_ok(), "use database: harus sukses");

        // buat table baru dengan kolom
        let raw_columns = vec![("id".to_string(), DataType::Int, vec![])];
        let res = dbm.execute(vec![CMD::CreateTable {
            table_name: TBL_NAME.to_string(),
            raw_columns,
        }]);
        debug_assert!(res.is_ok(), " create new table: harus sukses");

        // buat table kosong (tanpa kolom)
        let res = dbm.execute(vec![CMD::CreateTable {
            table_name: "table kosong".to_string(),
            raw_columns: vec![],
        }]);
        debug_assert!(res.is_err(), "create table no column: harus error");

        // rename table
        let res = dbm.execute(vec![CMD::RenameTable {
            old_name: TBL_NAME.to_string(),
            new_name: TBL_RENAMED.to_string(),
        }]);
        debug_assert!(res.is_ok(), "table rename: harus sukses");

        // disini TBL_NAME sudah tidak eksis.
        // yg terdaftar sekarang adalah TBL_RENAMED.

        // buat table baru dengan kolom
        let raw_columns = vec![("id".to_string(), DataType::Int, vec![])];
        let res = dbm.execute(vec![CMD::CreateTable {
            table_name: TBL_NAME.to_string(),
            raw_columns,
        }]);
        debug_assert!(res.is_ok(), " create new table 2: harus sukses");

        // ganti nama tabel menggunakan nama yg sudah ferdaftar.
        let res = dbm.execute(vec![CMD::RenameTable {
            old_name: TBL_NAME.to_string(),
            new_name: TBL_RENAMED.to_string(),
        }]);
        debug_assert!(res.is_err(), "table rename (duplicate name): harus error");

        // disini TBL_NAME masih eksis.
        // karena proses rename sebelumnya gagal diproses.

        // hapus table
        let res = dbm.execute(vec![CMD::DropTable {
            table_name: TBL_NAME.to_string(),
        }]);
        debug_assert!(res.is_ok(), "drop table eksis: harus sukses");

        Ok(())
    }

    // C.
    #[test]
    fn ddl_action_test() -> Result<(), DomainError> {
        remove_data_file()?;

        let mut dbm = DBM::new();
        // hapus data lama jika ada
        let res = dbm.execute(vec![CMD::CreateDatabase {
            db_name: DB_NAME.to_string(),
        }]);
        debug_assert!(res.is_ok(), "creat db: harus suskes");

        let res = dbm.execute(vec![CMD::UseDatabase {
            db_name: DB_NAME.to_string(),
        }]);
        debug_assert!(res.is_ok(), "use database: harus sukses");

        //tes invalid:  buat table duplicate column name
        let raw_columns = vec![
            ("name".to_string(), DataType::Text, vec![]),
            ("name".to_string(), DataType::Text, vec![]),
        ];
        let res = dbm.execute(vec![CMD::CreateTable {
            table_name: TBL_NAME.to_string(),
            raw_columns,
        }]);
        debug_assert!(
            res.is_err(),
            "harus gagal: create table duplicate column batch"
        );

        // buat table baru dengan kolom
        let raw_columns = vec![(
            "id".to_string(),
            DataType::Int,
            vec![
                ColumnConstraint::PrimaryKey,
                ColumnConstraint::Auto(Increment::Enabled { start: 1, step: 1 }),
            ],
        )];
        let res = dbm.execute(vec![CMD::CreateTable {
            table_name: TBL_NAME.to_string(),
            raw_columns,
        }]);
        debug_assert!(res.is_ok(), "harus aukses: add columns");

        // test invalid: add columns duplicate batch
        let raw_columns = vec![
            (
                "name".to_string(),
                DataType::Text,
                vec![ColumnConstraint::NotNull],
                ColumnPosition::Default,
            ),
            (
                "name".to_string(),
                DataType::Text,
                vec![ColumnConstraint::NotNull],
                ColumnPosition::Default,
            ),
        ];
        let res = dbm.execute(vec![CMD::AddColumns {
            table_name: TBL_NAME.to_string(),
            raw_columns,
        }]);
        debug_assert!(res.is_err(), "harus error: add column duplicate: {:?}", res);

        // add columns uniq
        let raw_columns = vec![
            (
                "name".to_string(),
                DataType::Text,
                vec![ColumnConstraint::NotNull],
                ColumnPosition::Default,
            ),
            (
                "status".to_string(),
                DataType::Enum {
                    name: "status".to_string(),
                    variants: vec!["lulus".to_string(), "gagal".to_string()],
                },
                vec![ColumnConstraint::NotNull],
                ColumnPosition::Default,
            ),
        ];
        let res = dbm.execute(vec![CMD::AddColumns {
            table_name: TBL_NAME.to_string(),
            raw_columns,
        }]);
        debug_assert!(res.is_ok(), "harus sukses: add column uniq");

        // disini column yg terdaftar id, name, status.

        // tes invalid: rename to non free namespace
        let res = dbm.execute(vec![CMD::RenameColumn {
            table_name: TBL_NAME.to_string(),
            old_name: "status".to_string(),
            new_name: "name".to_string(),
        }]);
        debug_assert!(res.is_err(), "harus error: rename to non free namespace");

        // rename to free namespace
        let res = dbm.execute(vec![CMD::RenameColumn {
            table_name: TBL_NAME.to_string(),
            old_name: "status".to_string(),
            new_name: "state".to_string(),
        }]);
        debug_assert!(res.is_ok(), "harus sukses: rename to free namespace");

        // tes invalid: drop column no active
        let res = dbm.execute(vec![CMD::DropColumn {
            table_name: TBL_NAME.to_string(),
            column_name: "no name".to_string(),
        }]);
        debug_assert!(res.is_err(), "harus error: drop column no active");

        // drop column active
        let res = dbm.execute(vec![CMD::DropColumn {
            table_name: TBL_NAME.to_string(),
            column_name: "state".to_string(),
        }]);
        debug_assert!(res.is_ok(), "harus sukses: drop column active");

        Ok(())
    }
}
