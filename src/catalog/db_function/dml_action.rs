use crate::BinaryOp;
use crate::catalog::table::Table;
use crate::domain::expr::{Expr, eval_expr, eval_where};
use crate::domain::id::{ColumnId, RowId};
use crate::domain::{AutoIncrement, DomainError, Row, SqlValue};
use std::collections::{HashMap, HashSet};
use std::ops::Bound;

/// Representasi Aksi Data Manipulation Language (DML).
#[derive(Debug, Clone, PartialEq)]
pub enum DmlAction {
    /// BULK INSERT: Menyisipkan satu atau beberapa baris data ke tabel.
    Insert { rows: Vec<Vec<SqlValue>> },

    /// UPDATE: Memperbarui nilai kolom berdasarkan kondisi predicate.
    Update {
        assignments: HashMap<ColumnId, Expr>,
        predicate: Option<Expr>,
    },

    /// DELETE: Menghapus baris berdasarkan kondisi predicate.
    Delete { predicate: Option<Expr> },
}

/// Hasil eksekusi operasi DML.
#[derive(Debug, Clone, PartialEq)]
pub enum DmlResult {
    /// Jumlah baris yang berhasil disisipkan.
    Inserted(usize),
    /// Jumlah baris yang berhasil diperbarui.
    Updated(usize),
    /// Jumlah baris yang berhasil dihapus.
    Deleted(usize),
}

struct StagedUpdate {
    row_idx: usize,
    row_id: RowId,
    old_entries: Vec<(ColumnId, SqlValue)>,
    new_entries: Vec<(ColumnId, SqlValue)>,
    new_row_values: Vec<SqlValue>,
}

/// Eksekutor terpusat untuk menjalankan operasi DML pada tabel.
pub(crate) fn execute_dml(table: &mut Table, action: &DmlAction) -> Result<DmlResult, DomainError> {
    match action {
        DmlAction::Insert { rows } => {
            let inserted_count = handle_insert(table, rows.clone())?;
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

/// Menerapkan penyisipan data (Insert) dengan Garansi Atomik All-or-Nothing.
fn handle_insert(table: &mut Table, raw_rows: Vec<Vec<SqlValue>>) -> Result<usize, DomainError> {
    if raw_rows.is_empty() {
        return Ok(0);
    }

    let columns = table.schema().columns().to_vec();
    let total_rows = raw_rows.len();

    struct StagedRow {
        assigned_row_id: RowId, // 👈 Simpan RowId yang dialokasikan resmi sejak awal
        prepared_values: Vec<SqlValue>,
        index_entries: Vec<(ColumnId, SqlValue)>,
    }

    let mut staged_rows = Vec::with_capacity(total_rows);
    let mut staged_counters = table.auto_increment_counters().clone();

    // 1. STAGING, ID ALLOCATION & VALIDATION PHASE
    for mut row_values in raw_rows {
        if row_values.len() < columns.len() {
            row_values.resize(columns.len(), SqlValue::Null);
        }

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

        table.schema().validate_row(&row_values)?;

        let index_entries: Vec<(ColumnId, SqlValue)> = columns
            .iter()
            .enumerate()
            .map(|(i, col)| (col.id, row_values[i].clone()))
            .collect();

        // 💡 Alokasikan RowId resmi di sini!
        let assigned_row_id = table.next_row_id();

        staged_rows.push(StagedRow {
            assigned_row_id,
            prepared_values: row_values,
            index_entries,
        });
    }

    // 2. DRY-RUN INDEXING PHASE
    for (offset, staged) in staged_rows.iter().enumerate() {
        if let Err(err) = table
            .index_registry_mut()
            .insert_entry(staged.assigned_row_id, &staged.index_entries)
        {
            // Rollback entri indeks yang terlanjur terpasang di iterasi sebelumnya
            for rb_staged in staged_rows[..offset].iter() {
                let _ = table
                    .index_registry_mut()
                    .remove_entry(rb_staged.assigned_row_id, &rb_staged.index_entries);
            }
            return Err(err);
        }
    }

    // 3. COMMIT PHASE
    *table.auto_increment_counters_mut() = staged_counters;

    for staged in staged_rows {
        // Gunakan RowId yang sudah di-stage & didaftarkan ke indeks!
        let row = Row::with_id(staged.assigned_row_id, staged.prepared_values);
        table.rows_mut().push(row);
    }

    Ok(total_rows)
}

/// Menerapkan penghapusan data (Delete) secara aman dan atomik.
fn handle_delete(table: &mut Table, predicate: Option<&Expr>) -> Result<usize, DomainError> {
    let columns = table.schema().columns().to_vec();
    let schema = table.schema().clone();

    // FAST PATH CHECK: Gunakan BTreeIndex Scan jika predicate mendukung indeks
    let candidate_row_ids: Option<HashSet<RowId>> =
        try_index_scan(table, predicate).map(|ids| ids.into_iter().collect());

    struct StagedDelete {
        row_idx: usize,
        row_id: RowId,
        index_entries: Vec<(ColumnId, SqlValue)>,
    }

    let mut staged_deletes = Vec::new();

    // 1. SCAN PHASE
    for (idx, row) in table.rows().iter().enumerate() {
        // Optimasi: Lewati evaluasi jika RowId tidak ada dalam kandidat BTreeIndex
        if let Some(ref valid_ids) = candidate_row_ids {
            if !valid_ids.contains(&row.id()) {
                continue;
            }
        }

        let matches_condition = match predicate {
            Some(expr) => eval_where(expr, &schema, row)?,
            None => true,
        };

        if matches_condition {
            let row_id = row.id();
            let index_entries: Vec<(ColumnId, SqlValue)> = columns
                .iter()
                .enumerate()
                .map(|(c_idx, col)| (col.id, row.values()[c_idx].clone()))
                .collect();

            staged_deletes.push(StagedDelete {
                row_idx: idx,
                row_id,
                index_entries,
            });
        }
    }

    if staged_deletes.is_empty() {
        return Ok(0);
    }

    // 2. STAGING INDEX REMOVAL (Atomic Transaction)
    let mut removed_count = 0;

    for staged in &staged_deletes {
        if let Err(err) = table
            .index_registry_mut()
            .remove_entry(staged.row_id, &staged.index_entries)
        {
            // Rollback hanya entri indeks yang sempat terhapus
            for rb_staged in staged_deletes[..removed_count].iter().rev() {
                let _ = table
                    .index_registry_mut()
                    .insert_entry(rb_staged.row_id, &rb_staged.index_entries);
            }
            return Err(err);
        }
        removed_count += 1;
    }

    // 3. COMMIT PHYSICAL ROWS (Hapus dari posisi indeks terbesar agar tidak merusak offset)
    let deleted_count = staged_deletes.len();
    for staged in staged_deletes.into_iter().rev() {
        table.rows_mut().remove(staged.row_idx);
    }

    Ok(deleted_count)
}

/// Menerapkan pembaruan data (Update) dengan penanganan `RowId` dan indeks yang konsisten.
fn handle_update(
    table: &mut Table,
    assignments: &HashMap<ColumnId, Expr>,
    predicate: Option<&Expr>,
) -> Result<usize, DomainError> {
    if assignments.is_empty() {
        return Ok(0);
    }

    let columns = table.schema().columns().to_vec();
    let schema = table.schema().clone();

    // FAST PATH CHECK: Gunakan BTreeIndex Scan jika predicate mendukung indeks
    let candidate_row_ids: Option<HashSet<RowId>> =
        try_index_scan(table, predicate).map(|ids| ids.into_iter().collect());

    let mut staged_updates = Vec::new();

    // 1. SCAN & STAGING PHASE
    for (idx, row) in table.rows().iter().enumerate() {
        // Optimasi: Lewati evaluasi jika RowId tidak ada dalam kandidat BTreeIndex
        if let Some(ref valid_ids) = candidate_row_ids {
            if !valid_ids.contains(&row.id()) {
                continue;
            }
        }

        let matches_condition = match predicate {
            Some(expr) => eval_where(expr, &schema, row)?,
            None => true,
        };

        if matches_condition {
            let row_id = row.id();
            let mut new_values = row.values().to_vec();
            let mut is_changed = false;

            for (col_idx, col) in columns.iter().enumerate() {
                if let Some(new_expr) = assignments.get(&col.id) {
                    let evaluated_val = eval_expr(new_expr, &schema, row)?;
                    if evaluated_val != new_values[col_idx] {
                        new_values[col_idx] = evaluated_val;
                        is_changed = true;
                    }
                }
            }

            // Jika tidak ada nilai yang berubah, lewati pemrosesan
            if !is_changed {
                continue;
            }

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

    // 2. STAGING INDEX UPDATE (Atomic Transaction)
    let mut modified_count = 0;

    for staged in &staged_updates {
        if let Err(err) = table
            .index_registry_mut()
            .remove_entry(staged.row_id, &staged.old_entries)
        {
            rollback_index_changes(table, &staged_updates[..modified_count]);
            return Err(err);
        }

        if let Err(err) = table
            .index_registry_mut()
            .insert_entry(staged.row_id, &staged.new_entries)
        {
            // Revert remove_entry untuk item yang gagal ini
            let _ = table
                .index_registry_mut()
                .insert_entry(staged.row_id, &staged.old_entries);
            rollback_index_changes(table, &staged_updates[..modified_count]);
            return Err(err);
        }

        modified_count += 1;
    }

    // 3. COMMIT PHYSICAL ROWS
    let updated_count = staged_updates.len();

    for staged in staged_updates {
        table.rows_mut()[staged.row_idx] = Row::with_id(staged.row_id, staged.new_row_values);
    }

    Ok(updated_count)
}

fn rollback_index_changes(table: &mut Table, processed_updates: &[StagedUpdate]) {
    for staged in processed_updates.iter().rev() {
        let _ = table
            .index_registry_mut()
            .remove_entry(staged.row_id, &staged.new_entries);
        let _ = table
            .index_registry_mut()
            .insert_entry(staged.row_id, &staged.old_entries);
    }
}

/// Mencoba mengekstrak kandidat RowId menggunakan BTreeIndex (O(log N)).
/// Mengembalikan `Some(Vec<RowId>)` jika klausa WHERE dapat dipetakan ke Indeks,
/// atau `None` jika harus memicu Full Table Scan.
pub(crate) fn try_index_scan(table: &Table, predicate: Option<&Expr>) -> Option<Vec<RowId>> {
    let expr = predicate?;

    match expr {
        // 1. EQUALITY: `WHERE col = val` atau `WHERE val = col`
        Expr::Binary {
            left,
            op: BinaryOp::Eq,
            right,
        } => {
            if let (Expr::Column(col_id), Expr::Literal(val)) = (left.as_ref(), right.as_ref()) {
                let index = table.index_registry().get_index(*col_id)?;
                return Some(index.lookup(val));
            }
            if let (Expr::Literal(val), Expr::Column(col_id)) = (left.as_ref(), right.as_ref()) {
                let index = table.index_registry().get_index(*col_id)?;
                return Some(index.lookup(val));
            }
            None
        }

        // 2. GREATER THAN: `WHERE col > val` atau `WHERE col >= val`
        Expr::Binary { left, op, right } if *op == BinaryOp::Gt || *op == BinaryOp::GtEq => {
            if let (Expr::Column(col_id), Expr::Literal(val)) = (left.as_ref(), right.as_ref()) {
                let index = table.index_registry().get_index(*col_id)?;
                let min_bound = if *op == BinaryOp::Gt {
                    Bound::Excluded(val)
                } else {
                    Bound::Included(val)
                };
                return Some(index.range_lookup(min_bound, Bound::Unbounded));
            }
            None
        }

        // 3. LESS THAN: `WHERE col < val` atau `WHERE col <= val`
        Expr::Binary { left, op, right } if *op == BinaryOp::Lt || *op == BinaryOp::LtEq => {
            if let (Expr::Column(col_id), Expr::Literal(val)) = (left.as_ref(), right.as_ref()) {
                let index = table.index_registry().get_index(*col_id)?;
                let max_bound = if *op == BinaryOp::Lt {
                    Bound::Excluded(val)
                } else {
                    Bound::Included(val)
                };
                return Some(index.range_lookup(Bound::Unbounded, max_bound));
            }
            None
        }

        // 4. IN LIST: `WHERE col IN (val1, val2, ...)`
        Expr::InList { expr, list } => {
            if let Expr::Column(col_id) = expr.as_ref() {
                let index = table.index_registry().get_index(*col_id)?;
                let mut matched_row_ids = HashSet::new();

                for item in list {
                    if let Expr::Literal(val) = item {
                        // Kumpulkan seluruh RowId dari setiap item list
                        for row_id in index.lookup(val) {
                            matched_row_ids.insert(row_id);
                        }
                    } else {
                        // Jika ada elemen list yang ekspresi non-literal, batalkan index scan
                        return None;
                    }
                }
                return Some(matched_row_ids.into_iter().collect());
            }
            None
        }

        _ => None,
    }
}
