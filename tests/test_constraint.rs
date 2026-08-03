// =============================================================================
// 1. UJI VALIDASI ATURAN SKEMA (DDL / Schema Staging Validation)
// =============================================================================

use sql::{
    AutoIncrement, BinaryOp, ColumnConstraint, ColumnDef, ColumnId, Database, DdlAction, DmlAction,
    DomainError, Expr, Schema, SqlType, SqlValue, TableConstraint,
};

#[test]
fn test_schema_duplicate_column_names_rejection() {
    let col1 = ColumnDef::new(ColumnId::from(1), "username", SqlType::Text);
    let col2 = ColumnDef::new(ColumnId::from(2), "USERNAME", SqlType::Text);

    // Duplikasi nama kolom (case-insensitive) harus ditolak
    let res = Schema::new(vec![col1, col2]);
    assert!(res.is_err());
    if let Err(DomainError::EvaluationError(msg)) = res {
        assert!(msg.contains("Duplikat nama kolom"));
    } else {
        panic!("Diharapkan DomainError::EvaluationError untuk nama kolom duplikat");
    }
}

#[test]
fn test_schema_autoincrement_must_be_int() {
    let auto_inc_cfg = AutoIncrement::Enabled { start: 1, step: 1 };
    let col = ColumnDef::with_constraints(
        ColumnId::from(1),
        "code",
        SqlType::Text, // AutoIncrement hanya diizinkan untuk SqlType::Int
        vec![ColumnConstraint::AutoIncrement(auto_inc_cfg)],
    );

    let res = Schema::new(vec![col]);
    assert!(res.is_err());
    if let Err(DomainError::EvaluationError(msg)) = res {
        assert!(msg.contains("hanya dapat digunakan untuk tipe data Int"));
    } else {
        panic!("Diharapkan error tipe data AutoIncrement");
    }
}

#[test]
fn test_schema_autoincrement_and_default_conflict() {
    let auto_inc_cfg = AutoIncrement::Enabled { start: 1, step: 1 };
    let col = ColumnDef::with_constraints(
        ColumnId::from(1),
        "id",
        SqlType::Int,
        vec![
            ColumnConstraint::AutoIncrement(auto_inc_cfg),
            ColumnConstraint::Default(SqlValue::Int(100)),
        ],
    );

    // AutoIncrement dan DEFAULT tidak boleh digunakan bersamaan pada satu kolom
    let res = Schema::new(vec![col]);
    assert!(res.is_err());
    if let Err(DomainError::EvaluationError(msg)) = res {
        assert!(msg.contains("tidak boleh memiliki AutoIncrement dan DEFAULT sekaligus"));
    } else {
        panic!("Diharapkan error konflik AutoIncrement dan DEFAULT");
    }
}

#[test]
fn test_schema_multiple_defaults_conflict() {
    let col = ColumnDef::with_constraints(
        ColumnId::from(1),
        "status",
        SqlType::Text,
        vec![
            ColumnConstraint::Default(SqlValue::Text("PENDING".into())),
            ColumnConstraint::Default(SqlValue::Text("ACTIVE".into())),
        ],
    );

    // Lebih dari satu constraint DEFAULT pada kolom harus ditolak
    let res = Schema::new(vec![col]);
    assert!(res.is_err());
    if let Err(DomainError::EvaluationError(msg)) = res {
        assert!(msg.contains("memiliki lebih dari satu constraint DEFAULT"));
    } else {
        panic!("Diharapkan error multiple DEFAULT");
    }
}

// =============================================================================
// 2. UJI VALIDASI BARIS DATA (Row Validation / DML Constraints)
// =============================================================================

#[test]
fn test_row_not_null_and_primary_key_constraint() {
    let col_id = ColumnDef::with_constraints(
        ColumnId::from(1),
        "id",
        SqlType::Int,
        vec![ColumnConstraint::PrimaryKey],
    );
    let col_name = ColumnDef::with_constraints(
        ColumnId::from(2),
        "name",
        SqlType::Text,
        vec![ColumnConstraint::NotNull],
    );

    let schema = Schema::new(vec![col_id, col_name]).unwrap();

    // 1. Valid Row
    let valid_row = vec![SqlValue::Int(1), SqlValue::Text("Budi".into())];
    assert!(schema.validate_row(&valid_row).is_ok());

    // 2. Gagal: Primary Key diisi NULL
    let invalid_pk = vec![SqlValue::Null, SqlValue::Text("Budi".into())];
    assert!(schema.validate_row(&invalid_pk).is_err());

    // 3. Gagal: Kolom NOT NULL diisi NULL
    let invalid_not_null = vec![SqlValue::Int(1), SqlValue::Null];
    let res = schema.validate_row(&invalid_not_null);
    assert!(res.is_err());
    if let Err(DomainError::EvaluationError(msg)) = res {
        assert!(msg.contains("tidak boleh NULL"));
    } else {
        panic!("Diharapkan error NOT NULL constraint");
    }
}

#[test]
fn test_row_type_mismatch() {
    let col_id = ColumnDef::new(ColumnId::from(1), "id", SqlType::Int);
    let col_age = ColumnDef::new(ColumnId::from(2), "age", SqlType::Int);

    let schema = Schema::new(vec![col_id, col_age]).unwrap();

    // Gagal: Memasukkan SqlValue::Text ke kolom bertipe SqlType::Int
    let invalid_row = vec![SqlValue::Int(1), SqlValue::Text("Dua Puluh".into())];
    let res = schema.validate_row(&invalid_row);
    assert!(matches!(res, Err(DomainError::TypeMismatch { .. })));
}

// =============================================================================
// 3. UJI CHECK CONSTRAINTS (Column Level & Table Level)
// =============================================================================

#[test]
fn test_column_check_constraint() {
    let col_id_id = ColumnId::from(1);
    let col_price_id = ColumnId::from(2);

    let col_id = ColumnDef::new(col_id_id, "id", SqlType::Int);

    // CHECK (price > 0)
    let check_expr = Expr::binary(Expr::col(col_price_id), BinaryOp::Gt, Expr::lit(0));
    let col_price = ColumnDef::with_constraints(
        col_price_id,
        "price",
        SqlType::Int,
        vec![ColumnConstraint::Check(check_expr)],
    );

    let schema = Schema::new(vec![col_id, col_price]).unwrap();

    // 1. Valid Row: price = 100 (> 0)
    assert!(
        schema
            .validate_row(&[SqlValue::Int(1), SqlValue::Int(100)])
            .is_ok()
    );

    // 2. Invalid Row: price = -50 (melanggar CHECK constraint)
    let res = schema.validate_row(&[SqlValue::Int(1), SqlValue::Int(-50)]);
    assert!(res.is_err());
    if let Err(DomainError::EvaluationError(msg)) = res {
        assert!(msg.contains("Pelanggaran CHECK constraint pada kolom"));
    } else {
        panic!("Diharapkan error CHECK constraint kolom");
    }
}

#[test]
fn test_table_check_constraint() {
    let col_start_id = ColumnId::from(1);
    let col_end_id = ColumnId::from(2);

    let col_start = ColumnDef::new(col_start_id, "start_val", SqlType::Int);
    let col_end = ColumnDef::new(col_end_id, "end_val", SqlType::Int);

    // Table Constraint: CHECK (end_val >= start_val)
    let table_check = TableConstraint::Check(Expr::binary(
        Expr::col(col_end_id),
        BinaryOp::GtEq,
        Expr::col(col_start_id),
    ));

    let schema =
        Schema::with_table_constraints(vec![col_start, col_end], vec![table_check]).unwrap();

    // 1. Valid: end_val (20) >= start_val (10)
    assert!(
        schema
            .validate_row(&[SqlValue::Int(10), SqlValue::Int(20)])
            .is_ok()
    );

    // 2. Invalid: end_val (5) < start_val (10)
    let res = schema.validate_row(&[SqlValue::Int(10), SqlValue::Int(5)]);
    assert!(res.is_err());
    if let Err(DomainError::EvaluationError(msg)) = res {
        assert!(msg.contains("Pelanggaran CHECK constraint pada tabel"));
    } else {
        panic!("Diharapkan error CHECK constraint tabel");
    }
}

// =============================================================================
// 4. INTEGRATION TEST VIA DATABASE API
// =============================================================================

#[test]
fn test_database_dml_constraint_enforcement() {
    let mut db = Database::default();

    // Buat tabel 'users' via DDL
    db.execute_ddl(DdlAction::CreateTable {
        name: "users".to_string(),
        columns: vec![
            (
                "id".to_string(),
                SqlType::Int,
                vec![ColumnConstraint::PrimaryKey],
            ),
            (
                "email".to_string(),
                SqlType::Text,
                vec![ColumnConstraint::NotNull],
            ),
        ],
    })
    .unwrap();

    // Coba INSERT nilai NULL ke kolom 'email' (NOT NULL)
    let insert_action = DmlAction::Insert {
        rows: vec![vec![SqlValue::Int(1), SqlValue::Null]],
    };

    let result = db.execute_dml("users", &insert_action);
    assert!(result.is_err());
}
