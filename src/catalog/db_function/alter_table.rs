use crate::catalog::database::Database;
use crate::domain::{ColumnConstraint, ColumnDef, DomainError, Schema, SqlType, SqlValue};
use std::collections::HashSet;

/// Representasi Aksi ALTER TABLE berstandar ANSI SQL
#[derive(Debug, Clone, PartialEq)]
pub enum AlterTableAction {
    AddColumn {
        name: String,
        sql_type: SqlType,
        constraints: Vec<ColumnConstraint>,
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

// impl AlterEngine {
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
            } => {
                execute_add_column(
                    &mut db_staging,
                    &current_table_name,
                    &name,
                    sql_type,
                    constraints,
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

    // 4. COMMIT: Jika SELURUH aksi berhasil, swap/apply staging ke db utama
    *db = db_staging;

    Ok(())
}

// --- PRIVATE HANDLER FUNCTIONS ---

fn execute_add_column(
    db: &mut Database,
    table_name: &str,
    col_name: &str,
    sql_type: SqlType,
    constraints: Vec<ColumnConstraint>,
) -> Result<(), DomainError> {
    let col_id = db.registry_mut().register_column(col_name);

    let new_col_def = ColumnDef::with_constraints(col_id, col_name, sql_type, constraints);
    let default_val = new_col_def
        .default_value()
        .cloned()
        .unwrap_or(SqlValue::Null);

    let table = db.get_table_mut(table_name)?;
    let mut new_columns = table.schema().columns().to_vec();
    new_columns.push(new_col_def);

    let new_schema = Schema::new(new_columns)?;
    *table.schema_mut() = new_schema;

    for row in table.rows_mut() {
        row.push(default_val.clone());
    }

    Ok(())
}

fn execute_drop_column(
    db: &mut Database,
    table_name: &str,
    col_name: &str,
) -> Result<(), DomainError> {
    let col_id = db.registry().get_column_id(col_name).ok_or_else(|| {
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

    let mut new_columns = table.schema().columns().to_vec();
    new_columns.remove(col_idx);
    let new_schema = Schema::new(new_columns)?;
    *table.schema_mut() = new_schema;

    for row in table.rows_mut() {
        row.remove(col_idx);
    }

    Ok(())
}

fn execute_rename_column(
    db: &mut Database,
    table_name: &str,
    old_name: &str,
    new_name: &str,
) -> Result<(), DomainError> {
    let col_id = db.registry().get_column_id(old_name).ok_or_else(|| {
        DomainError::EvaluationError(format!("Kolom '{old_name}' tidak ditemukan"))
    })?;

    db.registry_mut().rename_column(old_name, new_name)?;

    let table = db.get_table_mut(table_name)?;
    table.schema_mut().rename_column(col_id, new_name)?;

    Ok(())
}

fn execute_rename_table(
    db: &mut Database,
    old_name: &str,
    new_name: &str,
) -> Result<(), DomainError> {
    let table_id = db
        .registry()
        .get_table_id(old_name)
        .ok_or_else(|| DomainError::TableNotFound(old_name.to_string()))?;

    if db.registry().get_table_id(new_name).is_some() {
        return Err(DomainError::TableAlreadyExists(new_name.to_string()));
    }

    let table = db.tables_mut().get_mut(&table_id).unwrap();
    table.set_name(new_name);

    Ok(())
}

fn execute_modify_column_type(
    db: &mut Database,
    table_name: &str,
    col_name: &str,
    new_type: SqlType,
) -> Result<(), DomainError> {
    let col_id = db.registry().get_column_id(col_name).ok_or_else(|| {
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

    let mut new_values = Vec::with_capacity(table.rows().len());
    for row in table.rows() {
        let current_val = &row[col_idx];
        let casted_val = current_val.try_cast_to(&new_type)?;
        new_values.push(casted_val);
    }

    for (row, new_val) in table.rows_mut().iter_mut().zip(new_values) {
        row.values_mut()[col_idx] = new_val;
    }

    table.schema_mut().modify_column_type(col_id, new_type)?;

    Ok(())
}

fn execute_add_constraint(
    db: &mut Database,
    table_name: &str,
    col_name: &str,
    constraint: ColumnConstraint,
) -> Result<(), DomainError> {
    let col_id = db.registry().get_column_id(col_name).ok_or_else(|| {
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

    table
        .schema_mut()
        .add_column_constraint(col_id, constraint)?;

    Ok(())
}

fn execute_drop_constraint(
    db: &mut Database,
    table_name: &str,
    col_name: &str,
    constraint: &ColumnConstraint,
) -> Result<(), DomainError> {
    let col_id = db.registry().get_column_id(col_name).ok_or_else(|| {
        DomainError::EvaluationError(format!("Kolom '{col_name}' tidak ditemukan"))
    })?;

    let table = db.get_table_mut(table_name)?;
    table
        .schema_mut()
        .drop_column_constraint(col_id, constraint)?;

    Ok(())
}

fn execute_set_default(
    db: &mut Database,
    table_name: &str,
    col_name: &str,
    default_val: Option<SqlValue>,
) -> Result<(), DomainError> {
    let col_id = db.registry().get_column_id(col_name).ok_or_else(|| {
        DomainError::EvaluationError(format!("Kolom '{col_name}' tidak ditemukan"))
    })?;

    let table = db.get_table_mut(table_name)?;
    table.schema_mut().set_column_default(col_id, default_val)?;

    Ok(())
}
