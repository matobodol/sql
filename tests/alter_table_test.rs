use sql::catalog::database::Database;
use sql::db_function::ddl_action::AlterTableAction;
use sql::domain::sql_type::SqlType;
use sql::{ColumnConstraint, DdlAction, SqlValue};

/// Helper untuk membuat instance Database dengan 1 tabel awal 'users' dan 1 baris data
fn setup_test_db() -> Database {
    let mut db = Database::default();

    // Supaya ada data eksisting untuk menguji backfill/pembacaan row
    // db.create_table("users", vec![]).unwrap();
    db.execute_ddl(DdlAction::CreateTable {
        name: "users".into(),
        columns: vec![],
    })
    .unwrap();

    db
}

#[test]
fn test_multi_add_column_success() {
    let mut db = setup_test_db();

    // Skenario SUKSES: Menambahkan 3 kolom sekaligus dalam 1 kali transaksi ALTER TABLE
    let actions = vec![
        AlterTableAction::AddColumn {
            name: "id".to_string(),
            sql_type: SqlType::Int,
            constraints: vec![ColumnConstraint::PrimaryKey, ColumnConstraint::NotNull],
        },
        AlterTableAction::AddColumn {
            name: "name".to_string(),
            sql_type: SqlType::Text,
            constraints: vec![ColumnConstraint::NotNull],
        },
        AlterTableAction::AddColumn {
            name: "status".to_string(),
            sql_type: SqlType::Text,
            constraints: vec![],
        },
    ];

    // 1. Eksekusi multi-action
    let result = db.execute_alter("users", actions);
    assert!(
        result.is_ok(),
        "Eksekusi multi-action AddColumn harus sukses"
    );

    // 2. Verifikasi Skema: Harus memiliki tepat 3 kolom baru
    let table = db.get_table("users").expect("Tabel 'users' harus ada");
    let schema = table.schema();

    assert_eq!(schema.columns().len(), 3);
    assert_eq!(schema.columns()[0].name, "id");
    assert_eq!(schema.columns()[1].name, "name");
    assert_eq!(schema.columns()[2].name, "status");

    // 3. Verifikasi SymbolRegistry: Kolom-kolom baru harus terdaftar
    assert!(db.get_column_id("users", "id").is_some());
    assert!(db.get_column_id("users", "name").is_some());
    assert!(db.get_column_id("users", "status").is_some());
}

#[test]
fn test_multi_add_column_failure_rollback() {
    let mut db = setup_test_db();

    // Skenario GAGAL: Kolom 1 & 2 valid, tetapi Kolom 3 DUPLIKAT ('id' ditambahkan dua kali)
    let actions = vec![
        AlterTableAction::AddColumn {
            name: "id".to_string(),
            sql_type: SqlType::Int,
            constraints: vec![ColumnConstraint::PrimaryKey],
        },
        AlterTableAction::AddColumn {
            name: "name".to_string(),
            sql_type: SqlType::Text,
            constraints: vec![],
        },
        // 💥 Kolom ke-3 ini INVALID karena nama 'id' sudah dipakai pada aksi ke-1!
        AlterTableAction::AddColumn {
            name: "id".to_string(),
            sql_type: SqlType::Text,
            constraints: vec![],
        },
    ];

    // 1. Eksekusi multi-action (harus mengembalikan Error)
    let result = db.execute_alter("users", actions);
    assert!(
        result.is_err(),
        "Harus gagal karena ada aksi penambahan kolom duplikat"
    );

    // 2. VERIFIKASI ATOMISITAS (ROLLBACK):
    // Karena aksi ke-3 gagal, maka 'id' dan 'name' dari aksi ke-1 & ke-2 TIDAK BOLEH tersimpan di Schema!
    let table = db
        .get_table("users")
        .expect("Tabel 'users' harus tetap ada");
    let schema = table.schema();

    assert_eq!(
        schema.columns().len(),
        0,
        "Skema tabel harus tetap bersih (0 kolom) karena transaksi di-rollback!"
    );

    // 3. Verifikasi SymbolRegistry: Mapping ID untuk 'id' dan 'name' juga harus bersih/dibatalkan
    assert!(
        db.get_column_id("users", "id").is_none(),
        "'id' tidak boleh terdaftar di SymbolRegistry jika transaksi rollback"
    );
    assert!(
        db.get_column_id("users", "name").is_none(),
        "'name' tidak boleh terdaftar di SymbolRegistry jika transaksi rollback"
    );
}

#[test]
fn test_add_constraint_not_null_failure_and_rollback() {
    let mut db = Database::default();
    db.execute_ddl(DdlAction::CreateTable {
        name: "products".into(),
        columns: vec![],
    })
    .unwrap();

    // 1. Tambah kolom 'price' (opsional/nullable)
    db.execute_alter(
        "products",
        vec![AlterTableAction::AddColumn {
            name: "price".to_string(),
            sql_type: SqlType::Int,
            constraints: vec![],
        }],
    )
    .unwrap();

    // 2. Insert baris data yang berisi NULL pada price (melalui backfill / default null)
    // (Misal ada row eksisting yang harganya Null)

    // 3. Coba tambahkan constraint NOT NULL -> Harus Gagal jika data ber-NULL
    let _result = db.execute_alter(
        "products",
        vec![AlterTableAction::AddConstraint {
            col_name: "price".to_string(),
            constraint: ColumnConstraint::NotNull,
        }],
    );

    // Jika ada nilai Null, result harus Err
    // Dan state constraint pada schema harus di-rollback!
}

#[test]
fn test_set_and_drop_default_value() {
    let mut db = Database::default();
    // db.create_table("settings", vec![]).unwrap();
    db.execute_ddl(DdlAction::CreateTable {
        name: "settings".into(),
        columns: vec![],
    })
    .unwrap();

    // 1. Tambahkan kolom 'theme' tanpa default
    db.execute_alter(
        "settings",
        vec![AlterTableAction::AddColumn {
            name: "theme".to_string(),
            sql_type: SqlType::Text,
            constraints: vec![],
        }],
    )
    .unwrap();

    // 2. Pasang SetDefault -> 'dark'
    db.execute_alter(
        "settings",
        vec![AlterTableAction::SetDefault {
            col_name: "theme".to_string(),
            default_val: Some(SqlValue::Text("dark".to_string())),
        }],
    )
    .unwrap();

    let table = db.get_table("settings").unwrap();
    let col = &table.schema().columns()[0];
    assert!(
        col.constraints
            .contains(&ColumnConstraint::Default(SqlValue::Text(
                "dark".to_string()
            )))
    );

    // 3. Drop Default (SetDefault -> None)
    db.execute_alter(
        "settings",
        vec![AlterTableAction::SetDefault {
            col_name: "theme".to_string(),
            default_val: None,
        }],
    )
    .unwrap();

    let table = db.get_table("settings").unwrap();
    let col = &table.schema().columns()[0];
    assert!(
        !col.constraints
            .iter()
            .any(|c| matches!(c, ColumnConstraint::Default(_)))
    );
}
