use crate::catalog::database::Database;
use crate::domain::{
    ColumnConstraint, ColumnDef, ColumnId, DomainError, Schema, SqlType, SqlValue,
};
use crate::{Table, TableId};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq)]
pub enum ColumnPosition {
    Default,       // Ditaruh di paling akhir (standard ANSI)
    First,         // Ditaruh di paling awal (index 0)
    After(String), // Ditaruh setelah kolom tertentu
}

/// Enum tingkat tinggi untuk memisahkan jenis-jenis DDL (ANSI SQL)
#[derive(Debug, Clone, PartialEq)]
pub enum DdlAction {
    /// Membuat tabel baru
    CreateTable {
        name: String,
        columns: Vec<(String, SqlType, Vec<ColumnConstraint>)>,
    },
    /// Menghapus tabel
    DropTable { name: String },
    /// Mengubah struktur tabel eksisting
    AlterTable {
        name: String,
        actions: Vec<AlterTableAction>,
    },
}

/// Memberikan kemampuan eksekusi DdlAction langsung dari Facade Database
pub(crate) fn execute_ddl(db: &mut Database, action: DdlAction) -> Result<(), DomainError> {
    match action {
        DdlAction::CreateTable { name, columns } => {
            create_table(db, &name, columns)?;
            Ok(())
        }
        DdlAction::DropTable { name } => drop_table(db, &name),
        DdlAction::AlterTable { name, actions } => db.execute_alter(&name, actions),
    }
}

/// Representasi Aksi ALTER TABLE berstandar ANSI SQL.
#[derive(Debug, Clone, PartialEq)]
pub enum AlterTableAction {
    AddColumn {
        name: String,
        sql_type: SqlType,
        constraints: Vec<ColumnConstraint>,
        position: ColumnPosition,
    },
    DropColumn {
        name: String,
    },
    RenameColumn {
        old_name: String,
        new_name: String,
    },
    RenameTable {
        new_name: String,
    },
    ModifyColumnType {
        name: String,
        new_type: SqlType,
    },
    AddConstraint {
        col_name: String,
        constraint: ColumnConstraint,
    },
    DropConstraint {
        col_name: String,
        constraint: ColumnConstraint,
    },
    SetDefault {
        col_name: String,
        default_val: Option<SqlValue>,
    },
}

/// Menjalankan serangkaian aksi ALTER TABLE secara transaksi atomik (staging).
///
/// Jika salah satu aksi gagal, perubahan pada staging `db_staging` dibatalkan
/// dan tidak di-commit ke `db` utama.
pub(crate) fn execute_alter(
    db: &mut Database,
    table_name: &str,
    actions: Vec<AlterTableAction>,
) -> Result<(), DomainError> {
    // 1. Cek awal keberadaan tabel
    if db.get_table(table_name).is_err() {
        return Err(DomainError::TableNotFound(table_name.to_string()));
    }

    // 2. Buat snapshot staging (clone database state sementara)
    let mut db_staging = db.clone();
    let mut current_table_name = table_name.to_string();

    // 3. Jalankan seluruh aksi pada db_staging via pattern matching
    for action in actions {
        match action {
            AlterTableAction::AddColumn {
                name,
                sql_type,
                constraints,
                position,
            } => {
                execute_add_column(
                    &mut db_staging,
                    &current_table_name,
                    &name,
                    sql_type,
                    constraints,
                    position,
                )?;
            }
            AlterTableAction::DropColumn { name } => {
                execute_drop_column(&mut db_staging, &current_table_name, &name)?;
            }
            AlterTableAction::RenameColumn { old_name, new_name } => {
                execute_rename_column(&mut db_staging, &current_table_name, &old_name, &new_name)?;
            }
            AlterTableAction::RenameTable { new_name } => {
                execute_rename_table(&mut db_staging, &current_table_name, &new_name)?;
                current_table_name = new_name;
            }
            AlterTableAction::ModifyColumnType { name, new_type } => {
                execute_modify_column_type(&mut db_staging, &current_table_name, &name, new_type)?;
            }
            AlterTableAction::AddConstraint {
                col_name,
                constraint,
            } => {
                execute_add_constraint(
                    &mut db_staging,
                    &current_table_name,
                    &col_name,
                    constraint,
                )?;
            }
            AlterTableAction::DropConstraint {
                col_name,
                constraint,
            } => {
                execute_drop_constraint(
                    &mut db_staging,
                    &current_table_name,
                    &col_name,
                    &constraint,
                )?;
            }
            AlterTableAction::SetDefault {
                col_name,
                default_val,
            } => {
                execute_set_default(&mut db_staging, &current_table_name, &col_name, default_val)?;
            }
        }
    }

    // 4. COMMIT: Jika SELURUH aksi berhasil, swap staging ke db utama
    *db = db_staging;

    Ok(())
}

// --- PRIVATE HANDLER FUNCTIONS ---

/// Eksekutor internal untuk membuat tabel dan register.
fn create_table(
    db: &mut Database,
    table_name: &str,
    raw_columns: Vec<(String, SqlType, Vec<ColumnConstraint>)>,
) -> Result<TableId, DomainError> {
    // 1. Register tabel ke registry
    let table_id = db.registry_mut().register_table(table_name)?;

    // Helper closure untuk rollback registry jika terjadi error di tengah jalan
    let build_schema = || {
        let mut column_defs = Vec::with_capacity(raw_columns.len());
        for (col_name, sql_type, constraints) in raw_columns {
            let col_id = db.registry_mut().register_column(table_id, &col_name);
            column_defs.push(ColumnDef::with_constraints(
                col_id,
                col_name,
                sql_type,
                constraints,
            ));
        }

        Schema::new(column_defs)
    };

    // 2. Buat skema. Jika GAGAL, bersihkan registry agar tidak ada ID menggantung!
    let schema = match build_schema() {
        Ok(s) => s,
        Err(err) => {
            let _ = db.registry_mut().unregister_table(table_name);
            return Err(err);
        }
    };

    // 3. Simpan tabel ke HashMap
    let table = Table::new(table_id, table_name, schema);
    db.tables_mut().insert(table_id, table);

    Ok(table_id)
}

/// Eksekutor internal untuk menghapus tabel dan unregister dari registry.
fn drop_table(db: &mut Database, table_name: &str) -> Result<(), DomainError> {
    let table_id = db.registry_mut().unregister_table(table_name)?;
    db.tables_mut().remove(&table_id);
    Ok(())
}

/// Eksekutor internal untuk menambahkan kolom baru secara atomik.
/// Eksekutor internal untuk menambahkan kolom baru dengan opsi posisi (FIRST / AFTER / Default).
fn execute_add_column(
    db: &mut Database,
    table_name: &str,
    col_name: &str,
    sql_type: SqlType,
    constraints: Vec<ColumnConstraint>,
    position: ColumnPosition,
) -> Result<(), DomainError> {
    let table_id = db
        .registry()
        .get_table_id(table_name)
        .ok_or_else(|| DomainError::TableNotFound(table_name.to_string()))?;

    // 1. Read-Only Staging Schema & Tentukan Indeks Posisi
    let (mut staged_columns, target_idx) = {
        let table = db.get_table(table_name)?;
        let cols = table.schema().columns();

        let idx = match position {
            ColumnPosition::First => 0,
            ColumnPosition::After(ref ref_col_name) => {
                let ref_col_id = db
                    .registry()
                    .get_column_id(table_id, ref_col_name)
                    .ok_or_else(|| {
                        DomainError::EvaluationError(format!(
                            "Kolom referensi '{ref_col_name}' tidak ditemukan"
                        ))
                    })?;

                let pos = cols
                    .iter()
                    .position(|c| c.id == ref_col_id)
                    .ok_or_else(|| {
                        DomainError::EvaluationError(format!(
                            "Kolom '{ref_col_name}' tidak ada di skema"
                        ))
                    })?;

                pos + 1 // Sisipkan TEPAT SETELAH kolom referensi
            }
            ColumnPosition::Default => cols.len(), // Sisipkan di paling akhir
        };

        (cols.to_vec(), idx)
    };

    // 2. Gunakan Temp ID unik berbasis panjang skema
    let temp_id = ColumnId(u32::MAX - staged_columns.len() as u32);
    let dummy_col_def =
        ColumnDef::with_constraints(temp_id, col_name, sql_type.clone(), constraints.clone());

    // Insert dummy di target_idx
    staged_columns.insert(target_idx, dummy_col_def);

    // 3. Validasi Atomik Skema Baru
    Schema::validate_schema_columns(&staged_columns)?;

    // 4. COMMIT Phase: Ambil Real ID dari SymbolRegistry
    let col_id = db.registry_mut().register_column(table_id, col_name);
    let real_col_def = ColumnDef::with_constraints(col_id, col_name, sql_type, constraints);

    let default_val = real_col_def
        .default_value()
        .cloned()
        .unwrap_or(SqlValue::Null);

    // Ganti dummy_col_def pada posisi target_idx dengan real_col_def
    staged_columns[target_idx] = real_col_def;

    let new_schema = Schema::new(staged_columns)?;

    // 5. Update Schema & Data Baris (Rows)
    let table = db.get_table_mut(table_name)?;
    *table.schema_mut() = new_schema;

    // Sisipkan nilai default tepat di posisi `target_idx` untuk semua baris
    for row in table.rows_mut() {
        // Gunakan insert() alih-alih push()
        row.insert(target_idx, default_val.clone());
    }

    // 💡 SINKRONISASI INDEKS: Rebuild indeks setelah kolom dan baris diperbarui
    table.rebuild_indexes()?;

    Ok(())
}

/// Eksekutor internal untuk menghapus kolom dari tabel.
fn execute_drop_column(
    db: &mut Database,
    table_name: &str,
    col_name: &str,
) -> Result<(), DomainError> {
    let table_id = db
        .registry()
        .get_table_id(table_name)
        .ok_or_else(|| DomainError::TableNotFound(table_name.to_string()))?;

    let col_id = db
        .registry()
        .get_column_id(table_id, col_name)
        .ok_or_else(|| {
            DomainError::EvaluationError(format!("Kolom '{col_name}' tidak ditemukan"))
        })?;

    // 1. Dapatkan index kolom fisik
    let table = db.get_table_mut(table_name)?;
    let col_idx = table
        .schema()
        .columns()
        .iter()
        .position(|c| c.id == col_id)
        .ok_or_else(|| {
            DomainError::EvaluationError(format!("Kolom '{col_name}' tidak ada di tabel"))
        })?;

    // 2. Potong kolom dari Schema dan Rows
    let mut new_columns = table.schema().columns().to_vec();
    new_columns.remove(col_idx);
    let new_schema = Schema::new(new_columns)?;
    *table.schema_mut() = new_schema;

    for row in table.rows_mut() {
        row.remove(col_idx);
    }

    // 3. Wajib: Bersihkan dari SymbolRegistry agar ID tidak leaking!
    db.registry_mut().unregister_column(table_id, col_name)?;

    // 💡 SINKRONISASI INDEKS: Hapus indeks kolom yang di-drop & rebuild indeks tersisa
    let table = db.get_table_mut(table_name)?;
    table.index_registry_mut().drop_index(col_id);
    table.rebuild_indexes()?;

    Ok(())
}

/// Eksekutor internal untuk mengubah nama kolom.
fn execute_rename_column(
    db: &mut Database,
    table_name: &str,
    old_name: &str,
    new_name: &str,
) -> Result<(), DomainError> {
    let table_id = db
        .registry()
        .get_table_id(table_name)
        .ok_or_else(|| DomainError::TableNotFound(table_name.to_string()))?;

    let col_id = db
        .registry()
        .get_column_id(table_id, old_name)
        .ok_or_else(|| DomainError::ColumnNotFound(old_name.to_string()))?;

    // Ubah nama di Schema dulu (akan memicu validasi duplikasi nama baru)
    let table = db.get_table_mut(table_name)?;
    table.schema_mut().rename_column(col_id, new_name)?;

    // Jika Schema berhasil diperbarui, baru perbarui Registry
    db.registry_mut()
        .rename_column(table_id, old_name, new_name)?;

    Ok(())
}

/// Eksekutor internal untuk mengubah nama tabel.
/// Eksekutor internal untuk mengubah nama tabel.
fn execute_rename_table(
    db: &mut Database,
    old_name: &str,
    new_name: &str,
) -> Result<(), DomainError> {
    let table_id = db
        .registry()
        .get_table_id(old_name)
        .ok_or_else(|| DomainError::TableNotFound(old_name.to_string()))?;

    // 1. Perbarui SymbolRegistry
    db.registry_mut().rename_table(old_name, new_name)?;

    // 2. Perbarui nama pada object Table (jika HashMap berkey-kan TableId)
    if let Some(table) = db.tables_mut().get_mut(&table_id) {
        table.set_name(new_name);
    }

    Ok(())
}

/// Eksekutor internal untuk mengubah tipe data kolom dan melakukan konversi data baris.
fn execute_modify_column_type(
    db: &mut Database,
    table_name: &str,
    col_name: &str,
    new_type: SqlType,
) -> Result<(), DomainError> {
    // Validasi tipe baru (termasuk cek duplikasi varian enum)
    new_type.validate_enum_variants()?;

    let table_id = db
        .registry()
        .get_table_id(table_name)
        .ok_or_else(|| DomainError::TableNotFound(table_name.to_string()))?;

    let col_id = db
        .registry()
        .get_column_id(table_id, col_name)
        .ok_or_else(|| {
            DomainError::EvaluationError(format!("Kolom '{col_name}' tidak ditemukan"))
        })?;

    let table = db.get_table_mut(table_name)?;

    let col_idx = table
        .schema()
        .columns()
        .iter()
        .position(|c| c.id == col_id)
        .ok_or_else(|| {
            DomainError::EvaluationError(format!("Kolom '{col_name}' tidak ada di tabel"))
        })?;

    // Try Casting seluruh data baris
    let mut new_values = Vec::with_capacity(table.rows().len());
    for row in table.rows() {
        let current_val = &row[col_idx];
        let casted_val = current_val.try_cast_to(&new_type)?;
        new_values.push(casted_val);
    }

    // Terapkan tipe baru pada Schema (Memicu Re-validation Schema)
    table.schema_mut().modify_column_type(col_id, new_type)?;

    // Terapkan nilai hasil cast ke data baris
    for (row, new_val) in table.rows_mut().iter_mut().zip(new_values) {
        row.values_mut()[col_idx] = new_val;
    }

    // 💡 SINKRONISASI INDEKS: Rebuild indeks agar tipe data baru dalam B-Tree cocok
    table.rebuild_indexes()?;
    Ok(())
}

/// Eksekutor internal untuk menambahkan batasan/constraint baru pada kolom.
fn execute_add_constraint(
    db: &mut Database,
    table_name: &str,
    col_name: &str,
    constraint: ColumnConstraint,
) -> Result<(), DomainError> {
    let table_id = db
        .registry()
        .get_table_id(table_name)
        .ok_or_else(|| DomainError::TableNotFound(table_name.to_string()))?;

    let col_id = db
        .registry()
        .get_column_id(table_id, col_name)
        .ok_or_else(|| {
            DomainError::EvaluationError(format!("Kolom '{col_name}' tidak ditemukan"))
        })?;

    let table = db.get_table_mut(table_name)?;

    let col_idx = table
        .schema()
        .columns()
        .iter()
        .position(|c| c.id == col_id)
        .ok_or_else(|| {
            DomainError::EvaluationError(format!("Kolom '{col_name}' tidak ada di tabel"))
        })?;

    let column_def = &table.schema().columns()[col_idx];
    if column_def.constraints.contains(&constraint) {
        return Err(DomainError::EvaluationError(format!(
            "Constraint '{:?}' sudah ada pada kolom '{col_name}'",
            constraint
        )));
    }

    match &constraint {
        ColumnConstraint::NotNull => {
            for row in table.rows() {
                if row[col_idx].is_null() {
                    return Err(DomainError::EvaluationError(format!(
                        "Gagal menambahkan NOT NULL: terdapat nilai NULL pada kolom '{col_name}'"
                    )));
                }
            }
        }
        ColumnConstraint::Unique => {
            let mut seen_values = HashSet::new();
            for row in table.rows() {
                let val = &row[col_idx];
                if !val.is_null() {
                    if !seen_values.insert(val) {
                        return Err(DomainError::EvaluationError(format!(
                            "Gagal menambahkan UNIQUE: terdapat nilai duplikat '{:?}' pada kolom '{col_name}'",
                            val
                        )));
                    }
                }
            }
        }
        ColumnConstraint::PrimaryKey => {
            let mut seen_values = HashSet::new();
            for row in table.rows() {
                let val = &row[col_idx];
                if val.is_null() {
                    return Err(DomainError::EvaluationError(format!(
                        "Gagal menambahkan PRIMARY KEY: terdapat nilai NULL pada kolom '{col_name}'"
                    )));
                }
                if !seen_values.insert(val) {
                    return Err(DomainError::EvaluationError(format!(
                        "Gagal menambahkan PRIMARY KEY: terdapat nilai duplikat '{:?}' pada kolom '{col_name}'",
                        val
                    )));
                }
            }
        }
        ColumnConstraint::Default(_) => {}
        ColumnConstraint::AutoIncrement(_) => {
            for row in table.rows() {
                if let SqlValue::Int(_) = &row[col_idx] {
                } else if !row[col_idx].is_null() {
                    return Err(DomainError::EvaluationError(format!(
                        "Gagal menambahkan AutoIncrement: kolom '{col_name}' berisi tipe data non-Int"
                    )));
                }
            }
        }
        ColumnConstraint::Check(_) => {}
    }

    // Tambahkan constraint ke schema
    table
        .schema_mut()
        .add_column_constraint(col_id, constraint.clone())?;

    // 💡 Jika constraint adalah Unique/PrimaryKey, buat BTreeIndex & rebuild!
    if matches!(
        constraint,
        ColumnConstraint::Unique | ColumnConstraint::PrimaryKey
    ) {
        let _ = table.index_registry_mut().create_btree_index(col_id, true);
        table.rebuild_indexes()?;
    }

    Ok(())
}

/// Eksekutor internal untuk menghapus batasan/constraint dari kolom.
fn execute_drop_constraint(
    db: &mut Database,
    table_name: &str,
    col_name: &str,
    constraint: &ColumnConstraint,
) -> Result<(), DomainError> {
    let table_id = db
        .registry()
        .get_table_id(table_name)
        .ok_or_else(|| DomainError::TableNotFound(table_name.to_string()))?;

    let col_id = db
        .registry()
        .get_column_id(table_id, col_name)
        .ok_or_else(|| {
            DomainError::EvaluationError(format!("Kolom '{col_name}' tidak ditemukan"))
        })?;

    let table = db.get_table_mut(table_name)?;
    table
        .schema_mut()
        .drop_column_constraint(col_id, constraint)?;

    Ok(())
}

/// Eksekutor internal untuk mengatur nilai default kolom.
fn execute_set_default(
    db: &mut Database,
    table_name: &str,
    col_name: &str,
    default_val: Option<SqlValue>,
) -> Result<(), DomainError> {
    let table_id = db
        .registry()
        .get_table_id(table_name)
        .ok_or_else(|| DomainError::TableNotFound(table_name.to_string()))?;

    let col_id = db
        .registry()
        .get_column_id(table_id, col_name)
        .ok_or_else(|| {
            DomainError::EvaluationError(format!("Kolom '{col_name}' tidak ditemukan"))
        })?;

    let table = db.get_table_mut(table_name)?;
    table.schema_mut().set_column_default(col_id, default_val)?;

    Ok(())
}
