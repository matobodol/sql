use std::collections::{HashMap, HashSet};

use crate::catalog::CatalogStore;
use crate::expression::{eval_expr, eval_where};
use crate::index::{Index, IndexRegistry};
use crate::validator::validate_row;
use crate::{BinaryOp, ColumnId, DomainError, Expr, RowId, Schema, ValueType};
use crate::{BufferPoolManager, RID, Row, TableHeap, TableId};

struct StagedUpdate {
    #[allow(warnings)]
    row_idx: usize,
    rid: RID,
    old_entries: Vec<(ColumnId, ValueType)>,
    new_entries: Vec<(ColumnId, ValueType)>,
    new_row_values: Vec<ValueType>,
}

pub(crate) fn handle_insert(
    catalog: &CatalogStore,
    table_heap: &mut TableHeap,
    bpm: &mut BufferPoolManager,
    index_registry: &mut IndexRegistry,
    auto_increment_counters: &mut HashMap<ColumnId, i64>,
    table_id: TableId,
    raw_rows: Vec<Vec<ValueType>>,
) -> Result<usize, DomainError> {
    if raw_rows.is_empty() {
        return Ok(0);
    }

    let schema = catalog.get_schema(table_id)?;
    let columns = schema.columns();
    let total_rows = raw_rows.len();

    // Identifikasi kolom yang memiliki indeks
    let indexed_col_indices: Vec<(usize, ColumnId)> = columns
        .iter()
        .enumerate()
        .filter(|(_, col)| index_registry.has_index(col.id))
        .map(|(idx, col)| (idx, col.id))
        .collect();

    struct StagedRow {
        prepared_values: Vec<ValueType>,
        index_entries: Vec<(ColumnId, ValueType)>,
    }

    let mut staged_rows = Vec::with_capacity(total_rows);
    for mut row_values in raw_rows {
        if row_values.len() < columns.len() {
            row_values.resize(columns.len(), ValueType::Null);
        }

        for (i, col) in columns.iter().enumerate() {
            let is_null = row_values[i].is_null();
            if col.is_auto_increment() && is_null {
                // UBAH `staged_counters` MENJADI `auto_increment_counters` yang dikirim dari TableContext
                let counter = auto_increment_counters.entry(col.id).or_insert(1);
                row_values[i] = ValueType::Int(*counter);
                *counter += 1;
            } else if is_null {
                if let Some(default_val) = col.default_value() {
                    row_values[i] = default_val.clone();
                }
            }
        }

        validate_row(&schema, &row_values)?;

        let index_entries: Vec<(ColumnId, ValueType)> = indexed_col_indices
            .iter()
            .map(|&(c_idx, col_id)| (col_id, row_values[c_idx].clone()))
            .collect();

        staged_rows.push(StagedRow {
            prepared_values: row_values,
            index_entries,
        });
    }

    // Lakukan commit ke TableHeap satu per satu, lalu daftarkan ke Indeks B-Tree
    let mut inserted_rids = Vec::with_capacity(total_rows);

    for (offset, staged) in staged_rows.iter().enumerate() {
        let row_bytes = bincode::serialize(&staged.prepared_values)
            .map_err(|e| DomainError::storage(e.to_string()))?;

        // Simpan ke halaman fisik disk via TableHeap
        let rid = table_heap.insert_tuple(bpm, &row_bytes)?;
        inserted_rids.push((rid, &staged.index_entries));

        let entries_ref: Vec<(ColumnId, &ValueType)> = staged
            .index_entries
            .iter()
            .map(|(col_id, val)| (*col_id, val))
            .collect();

        // Masukkan ke B-Tree Index menggunakan RowId yang merepresentasikan RID (slot_id)
        let row_id_alias = RowId(rid.slot_id as u64);
        if let Err(err) = index_registry.insert_entry_ref(row_id_alias, &entries_ref) {
            // Rollback indeks dan hapus tuple fisik yang sudah terlanjur masuk jika terjadi pelanggaran unik
            for (rb_rid, rb_entries) in inserted_rids[..offset].iter() {
                let rb_entries_ref: Vec<(ColumnId, &ValueType)> = rb_entries
                    .iter()
                    .map(|(col_id, val)| (*col_id, val))
                    .collect();
                let _ =
                    index_registry.remove_entry_ref(RowId(rb_rid.slot_id as u64), &rb_entries_ref);
                let _ = table_heap.delete_tuple(bpm, *rb_rid);
            }
            let _ = table_heap.delete_tuple(bpm, rid);
            return Err(err);
        }
    }

    Ok(total_rows)
}

pub(crate) fn handle_delete(
    catalog: &CatalogStore,
    table_heap: &mut TableHeap,
    bpm: &mut BufferPoolManager,
    index_registry: &mut IndexRegistry,
    table_id: TableId,
    predicate: Option<&Expr>,
) -> Result<usize, DomainError> {
    let schema_cols = catalog.get_schema_columns(table_id)?;
    let schema = Schema::new(schema_cols.to_vec())?;
    let columns = schema.columns();

    // Coba optimasi pencarian menggunakan Index Scan jika predikat mendukung
    let candidate_rids: Option<HashSet<RID>> = try_index_scan(index_registry, &schema, predicate)
        .map(|ids| ids.into_iter().map(|id| RID::from(id)).collect()); // Sesuaikan mapping RID Anda

    let indexed_col_indices: Vec<(usize, ColumnId)> = columns
        .iter()
        .enumerate()
        .filter(|(_, col)| index_registry.has_index(col.id))
        .map(|(idx, col)| (idx, col.id))
        .collect();

    struct StagedDelete {
        rid: RID,
        index_entries: Vec<(ColumnId, ValueType)>,
    }

    let mut staged_deletes = Vec::new();
    let rids = table_heap.scan_rids(bpm)?;

    for rid in rids {
        if let Some(ref valid_rids) = candidate_rids {
            if !valid_rids.contains(&rid) {
                continue;
            }
        }

        if let Some(tuple_bytes) = table_heap.get_tuple(bpm, rid)? {
            let row_values: Vec<ValueType> = bincode::deserialize(&tuple_bytes)
                .map_err(|e| DomainError::storage(e.to_string()))?;
            let row = Row::with_id(RowId(rid.slot_id as u64), row_values);

            let matches_condition = match predicate {
                Some(expr) => eval_where(expr, &row)?,
                None => true,
            };

            if matches_condition {
                let index_entries: Vec<(ColumnId, ValueType)> = indexed_col_indices
                    .iter()
                    .map(|&(c_idx, col_id)| (col_id, row.values()[c_idx].clone()))
                    .collect();

                staged_deletes.push(StagedDelete { rid, index_entries });
            }
        }
    }

    if staged_deletes.is_empty() {
        return Ok(0);
    }

    let mut removed_count = 0;

    for staged in &staged_deletes {
        let entries_ref: Vec<(ColumnId, &ValueType)> = staged
            .index_entries
            .iter()
            .map(|(col_id, val)| (*col_id, val))
            .collect();

        let row_id_alias = RowId(staged.rid.slot_id as u64);
        if let Err(err) = index_registry.remove_entry_ref(row_id_alias, &entries_ref) {
            for rb_staged in staged_deletes[..removed_count].iter().rev() {
                let rb_entries_ref: Vec<(ColumnId, &ValueType)> = rb_staged
                    .index_entries
                    .iter()
                    .map(|(col_id, val)| (*col_id, val))
                    .collect();
                let _ = index_registry
                    .insert_entry_ref(RowId(rb_staged.rid.slot_id as u64), &rb_entries_ref);
            }
            return Err(err);
        }

        table_heap.delete_tuple(bpm, staged.rid)?;
        removed_count += 1;
    }

    Ok(staged_deletes.len())
}

pub(crate) fn handle_update(
    catalog: &CatalogStore,
    table_heap: &mut TableHeap,
    bpm: &mut BufferPoolManager,
    index_registry: &mut IndexRegistry,
    table_id: TableId,
    assignments: &HashMap<ColumnId, Expr>,
    predicate: Option<&Expr>,
) -> Result<usize, DomainError> {
    if assignments.is_empty() {
        return Ok(0);
    }

    let schema_cols = catalog.get_schema_columns(table_id)?;
    let schema = Schema::new(schema_cols.to_vec())?;
    let columns = schema.columns();

    let candidate_rids: Option<HashSet<RID>> = try_index_scan(index_registry, &schema, predicate)
        .map(|ids| ids.into_iter().map(|id| RID::from(id)).collect());

    let indexed_col_indices: Vec<(usize, ColumnId)> = columns
        .iter()
        .enumerate()
        .filter(|(_, col)| index_registry.has_index(col.id))
        .map(|(idx, col)| (idx, col.id))
        .collect();

    let mut staged_updates = Vec::new();
    let rids = table_heap.scan_rids(bpm)?;

    for rid in rids {
        if let Some(ref valid_rids) = candidate_rids {
            if !valid_rids.contains(&rid) {
                continue;
            }
        }

        if let Some(tuple_bytes) = table_heap.get_tuple(bpm, rid)? {
            let mut row_values: Vec<ValueType> = bincode::deserialize(&tuple_bytes)
                .map_err(|e| DomainError::storage(e.to_string()))?;
            let row = Row::with_id(RowId(rid.slot_id as u64), row_values.clone());

            let matches_condition = match predicate {
                Some(expr) => eval_where(expr, &row)?,
                None => true,
            };

            if matches_condition {
                let mut is_changed = false;

                for (col_idx, col) in columns.iter().enumerate() {
                    if let Some(new_expr) = assignments.get(&col.id) {
                        let evaluated_cow = eval_expr(new_expr, &row)?;
                        let evaluated_val = evaluated_cow.into_owned();

                        if evaluated_val != row_values[col_idx] {
                            row_values[col_idx] = evaluated_val;
                            is_changed = true;
                        }
                    }
                }

                if !is_changed {
                    continue;
                }

                validate_row(&schema, &row_values)?;

                let old_entries: Vec<(ColumnId, ValueType)> = indexed_col_indices
                    .iter()
                    .map(|&(c_idx, col_id)| (col_id, row.values()[c_idx].clone()))
                    .collect();

                let new_entries: Vec<(ColumnId, ValueType)> = indexed_col_indices
                    .iter()
                    .map(|&(c_idx, col_id)| (col_id, row_values[c_idx].clone()))
                    .collect();

                staged_updates.push(StagedUpdate {
                    row_idx: 0,
                    rid,
                    old_entries,
                    new_entries,
                    new_row_values: row_values,
                });
            }
        }
    }

    if staged_updates.is_empty() {
        return Ok(0);
    }

    let mut modified_count = 0;

    for staged in &staged_updates {
        let old_entries_ref: Vec<(ColumnId, &ValueType)> = staged
            .old_entries
            .iter()
            .map(|(col_id, val)| (*col_id, val))
            .collect();
        let new_entries_ref: Vec<(ColumnId, &ValueType)> = staged
            .new_entries
            .iter()
            .map(|(col_id, val)| (*col_id, val))
            .collect();

        let row_id_alias = RowId(staged.rid.slot_id as u64);

        if let Err(err) = index_registry.remove_entry_ref(row_id_alias, &old_entries_ref) {
            rollback_index_changes(index_registry, &staged_updates[..modified_count]);
            return Err(err);
        }

        if let Err(err) = index_registry.insert_entry_ref(row_id_alias, &new_entries_ref) {
            let _ = index_registry.insert_entry_ref(row_id_alias, &old_entries_ref);
            rollback_index_changes(index_registry, &staged_updates[..modified_count]);
            return Err(err);
        }

        // Perbarui data fisik di TableHeap
        table_heap.delete_tuple(bpm, staged.rid)?;
        let new_bytes = bincode::serialize(&staged.new_row_values)
            .map_err(|e| DomainError::storage(e.to_string()))?;
        table_heap.insert_tuple(bpm, &new_bytes)?;

        modified_count += 1;
    }

    Ok(staged_updates.len())
}

fn rollback_index_changes(index_registry: &mut IndexRegistry, processed_updates: &[StagedUpdate]) {
    for staged in processed_updates.iter().rev() {
        let old_entries_ref: Vec<(ColumnId, &ValueType)> = staged
            .old_entries
            .iter()
            .map(|(col_id, val)| (*col_id, val))
            .collect();
        let new_entries_ref: Vec<(ColumnId, &ValueType)> = staged
            .new_entries
            .iter()
            .map(|(col_id, val)| (*col_id, val))
            .collect();

        let row_id_alias = RowId(staged.rid.slot_id as u64);
        let _ = index_registry.remove_entry_ref(row_id_alias, &new_entries_ref);
        let _ = index_registry.insert_entry_ref(row_id_alias, &old_entries_ref);
    }
}

pub(crate) fn try_index_scan(
    index_registry: &IndexRegistry,
    schema: &Schema,
    predicate: Option<&Expr>,
) -> Option<Vec<RowId>> {
    let expr = predicate?;

    match expr {
        Expr::Binary {
            left,
            op: BinaryOp::Eq,
            right,
        } => {
            if let (Expr::Column(col_name), Expr::Literal(val)) = (left.as_ref(), right.as_ref()) {
                if let Some(col_id) = schema.get_column_by_name(col_name).map(|c| c.id) {
                    if let Some(index_impl) = index_registry.get_index(col_id) {
                        return Some(index_impl.lookup(val).to_vec());
                    }
                }
            }
            None
        }
        _ => None,
    }
}
