use crate::catalog::table::Table;
use crate::domain::expr::{Expr, eval_expr, eval_where};
use crate::domain::id::{ColumnId, RowId};
use crate::domain::{AutoIncrement, DomainError, Row, SqlValue};
use std::collections::HashMap;

/// Representasi Aksi Data Manipulation Language (DML)
#[derive(Debug, Clone, PartialEq)]
pub enum DmlAction {
    /// BULK INSERT: INSERT INTO table (rows...)
    Insert { rows: Vec<Vec<SqlValue>> },

    /// UPDATE table SET col1 = expr1, col2 = expr2 WHERE predicate
    Update {
        assignments: HashMap<ColumnId, Expr>,
        predicate: Option<Expr>,
    },

    /// DELETE FROM table WHERE predicate
    Delete { predicate: Option<Expr> },
}

/// Hasil eksekusi operasi DML
#[derive(Debug, Clone, PartialEq)]
pub enum DmlResult {
    Inserted(usize),
    Updated(usize),
    Deleted(usize),
}

/// Eksekutor terpusat untuk DML Action
pub(crate) fn execute_dml(table: &mut Table, action: DmlAction) -> Result<DmlResult, DomainError> {
    match action {
        DmlAction::Insert { rows } => {
            let inserted_count = handle_insert(table, rows)?;
            Ok(DmlResult::Inserted(inserted_count))
        }
        DmlAction::Update {
            assignments,
            predicate,
        } => {
            let updated_count = handle_update(table, &assignments, predicate.as_ref())?;
            Ok(DmlResult::Updated(updated_count))
        }
        DmlAction::Delete { predicate } => {
            let deleted_count = handle_delete(table, predicate.as_ref())?;
            Ok(DmlResult::Deleted(deleted_count))
        }
    }
}

// --- PRIVATE HANDLERS ---

/// Menerapkan Multiple Insert dengan Garansi All-or-Nothing (Atomic Staging)
fn handle_insert(table: &mut Table, raw_rows: Vec<Vec<SqlValue>>) -> Result<usize, DomainError> {
    if raw_rows.is_empty() {
        return Ok(0);
    }

    let columns = table.schema().columns().to_vec();
    let total_rows = raw_rows.len();

    // Staging struct untuk menampung baris data yang sudah tervalidasi & siap commit
    struct StagedRow {
        row_id: RowId,
        prepared_values: Vec<SqlValue>,
        index_entries: Vec<(ColumnId, SqlValue)>,
    }

    let mut staged_rows = Vec::with_capacity(total_rows);

    // Kloning state auto-increment counter untuk staging phase
    let mut staged_counters = table.auto_increment_counters().clone();
    let mut next_row_id = table.next_row_id();

    // ==========================================
    // PHASE 1: STAGING & VALIDASI (ALL-OR-NOTHING)
    // ==========================================
    for mut row_values in raw_rows {
        // 1. Pad array jika nilainya kurang dari jumlah kolom di schema
        if row_values.len() < columns.len() {
            row_values.resize(columns.len(), SqlValue::Null);
        }

        // 2. Transformasi AutoIncrement & Default Value (di Staging Counter)
        for (i, col) in columns.iter().enumerate() {
            let is_null = row_values[i].is_null();

            if col.is_auto_increment() && is_null {
                let counter = staged_counters
                    .get_mut(&col.id)
                    .expect("Counter auto-increment harusnya terinisialisasi");

                row_values[i] = SqlValue::Int(*counter);

                let step = match col.auto_increment_config() {
                    Some(AutoIncrement::Enabled { step, .. }) => *step,
                    _ => 1,
                };
                *counter += step;
            } else if col.is_auto_increment() && !is_null {
                if let SqlValue::Int(manual_val) = row_values[i] {
                    if let Some(counter) = staged_counters.get_mut(&col.id) {
                        if manual_val >= *counter {
                            let step = match col.auto_increment_config() {
                                Some(AutoIncrement::Enabled { step, .. }) => *step,
                                _ => 1,
                            };
                            *counter = manual_val + step;
                        }
                    }
                }
            } else if is_null {
                if let Some(default_val) = col.default_value() {
                    row_values[i] = default_val.clone();
                }
            }
        }

        // 3. Validasi Schema & Constraints
        table.schema().validate_row(&row_values)?;

        let staged_row_id = RowId::from(next_row_id);
        next_row_id += 1;

        let index_entries: Vec<(ColumnId, SqlValue)> = columns
            .iter()
            .enumerate()
            .map(|(i, col)| (col.id, row_values[i].clone()))
            .collect();

        staged_rows.push(StagedRow {
            row_id: staged_row_id,
            prepared_values: row_values,
            index_entries,
        });
    }

    // Dynamic dry-run ke IndexRegistry (Cek Unique Violation antar row di batch & terhadap DB)
    let mut rolled_back_entries: Vec<(RowId, Vec<(ColumnId, SqlValue)>)> = Vec::new();

    for staged in &staged_rows {
        if let Err(err) = table
            .index_registry_mut()
            .insert_entry(staged.row_id, &staged.index_entries)
        {
            // Jika 1 row saja gagal di dry-run, bersihkan index yang terlanjur terpasang di dry-run
            for (rb_id, rb_entries) in rolled_back_entries {
                let _ = table.index_registry_mut().remove_entry(rb_id, &rb_entries);
            }
            return Err(err);
        }
        rolled_back_entries.push((staged.row_id, staged.index_entries.clone()));
    }

    // ==========================================
    // PHASE 2: COMMIT PHASE (GARANSI PASTI LOLOS)
    // ==========================================
    // Commit AutoIncrement Counter & RowId
    *table.auto_increment_counters_mut() = staged_counters;

    for staged in staged_rows {
        let row = Row::new(staged.prepared_values);
        table.rows_mut().push(row);
        table.increment_next_row_id();
    }

    Ok(total_rows)
}

fn handle_update(
    table: &mut Table,
    assignments: &HashMap<ColumnId, Expr>,
    predicate: Option<&Expr>,
) -> Result<usize, DomainError> {
    let columns = table.schema().columns().to_vec();
    let schema = table.schema().clone();

    // Struct penampung staging perubahan per baris
    struct StagedUpdate {
        row_idx: usize,
        row_id: RowId,
        old_entries: Vec<(ColumnId, SqlValue)>,
        new_entries: Vec<(ColumnId, SqlValue)>,
        new_row_values: Vec<SqlValue>,
    }

    let mut staged_updates = Vec::new();

    // ==========================================
    // PHASE 1: EVALUASI & STAGING (ALL-OR-NOTHING)
    // ==========================================
    for (idx, row) in table.rows().iter().enumerate() {
        let matches_condition = match predicate {
            Some(expr) => eval_where(expr, &schema, row)?,
            None => true,
        };

        if matches_condition {
            let row_id = RowId::from((idx + 1) as u64);
            let mut new_values = row.values().to_vec();

            // 1. Evaluasi ekspresi assignment
            for (col_idx, col) in columns.iter().enumerate() {
                if let Some(new_expr) = assignments.get(&col.id) {
                    let evaluated_val = eval_expr(new_expr, &schema, row)?;
                    new_values[col_idx] = evaluated_val;
                }
            }

            // 2. Validasi skema (NOT NULL, CHECK, Types)
            schema.validate_row(&new_values)?;

            let old_entries: Vec<(ColumnId, SqlValue)> = columns
                .iter()
                .enumerate()
                .map(|(c_idx, col)| (col.id, row.values()[c_idx].clone()))
                .collect();

            let new_entries: Vec<(ColumnId, SqlValue)> = columns
                .iter()
                .enumerate()
                .map(|(c_idx, col)| (col.id, new_values[c_idx].clone()))
                .collect();

            staged_updates.push(StagedUpdate {
                row_idx: idx,
                row_id,
                old_entries,
                new_entries,
                new_row_values: new_values,
            });
        }
    }

    if staged_updates.is_empty() {
        return Ok(0);
    }

    // 3. Dry-Run Index Update (Simulasi pergantian indeks)
    // Mencabut indeks lama & memasukkan indeks baru secara temporary
    let mut modified_indexes = Vec::new();

    for staged in &staged_updates {
        // Cabut entri lama
        if let Err(err) = table
            .index_registry_mut()
            .remove_entry(staged.row_id, &staged.old_entries)
        {
            // Rollback indeks yang sudah terlanjur diubah di dry-run
            rollback_index_changes(table, modified_indexes);
            return Err(err);
        }

        // Pasang entri baru (Cek UNIQUE constraint)
        if let Err(err) = table
            .index_registry_mut()
            .insert_entry(staged.row_id, &staged.new_entries)
        {
            // Revert row ini
            let _ = table
                .index_registry_mut()
                .insert_entry(staged.row_id, &staged.old_entries);
            // Rollback seluruh indeks sebelumnya
            rollback_index_changes(table, modified_indexes);
            return Err(err);
        }

        modified_indexes.push((staged.row_id, &staged.old_entries, &staged.new_entries));
    }

    // ==========================================
    // PHASE 2: COMMIT PHYSICAL ROWS (PASTI SUCCESS)
    // ==========================================
    let updated_count = staged_updates.len();

    for staged in staged_updates {
        table.rows_mut()[staged.row_idx] = Row::new(staged.new_row_values);
    }

    Ok(updated_count)
}

/// Helper internal untuk mengembalikan kondisi Indeks jika Staging UPDATE gagal
fn rollback_index_changes(
    table: &mut Table,
    modified_indexes: Vec<(
        RowId,
        &Vec<(ColumnId, SqlValue)>,
        &Vec<(ColumnId, SqlValue)>,
    )>,
) {
    for (row_id, old_entries, new_entries) in modified_indexes.into_iter().rev() {
        let _ = table.index_registry_mut().remove_entry(row_id, new_entries);
        let _ = table.index_registry_mut().insert_entry(row_id, old_entries);
    }
}

fn handle_delete(table: &mut Table, predicate: Option<&Expr>) -> Result<usize, DomainError> {
    let columns = table.schema().columns().to_vec();
    let schema = table.schema().clone();
    let mut rows_to_delete = Vec::new();

    for (idx, row) in table.rows().iter().enumerate() {
        let matches_condition = match predicate {
            Some(expr) => eval_where(expr, &schema, row)?,
            None => true,
        };

        if matches_condition {
            let row_id = RowId::from((idx + 1) as u64);

            let index_entries: Vec<(ColumnId, SqlValue)> = columns
                .iter()
                .enumerate()
                .map(|(c_idx, col)| (col.id, row.values()[c_idx].clone()))
                .collect();

            rows_to_delete.push((idx, row_id, index_entries));
        }
    }

    let deleted_count = rows_to_delete.len();

    // Hapus dari indeks & hapus baris fisik dari posisi belakang agar indeks array tidak geser
    for (idx, row_id, index_entries) in rows_to_delete.into_iter().rev() {
        table
            .index_registry_mut()
            .remove_entry(row_id, &index_entries)?;
        table.rows_mut().remove(idx);
    }

    Ok(deleted_count)
}
