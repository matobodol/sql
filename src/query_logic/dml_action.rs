use std::collections::{HashMap, HashSet};
use std::ops::Bound;
use std::sync::Arc;

use crate::table_store::TableStorage;
use crate::{
    AutoIncrement, BinaryOp, ColumnId, Database, DomainError, Expr, RowId, Schema, SqlValue,
    eval_expr, eval_where,
};

struct StagedUpdate {
    row_idx: usize,
    row_id: RowId,
    old_entries: Vec<(ColumnId, SqlValue)>,
    new_entries: Vec<(ColumnId, SqlValue)>,
    new_row_values: Vec<SqlValue>,
}

pub(crate) fn handle_insert(
    db: &mut Database,
    table_name: &str,
    raw_rows: Vec<Vec<SqlValue>>,
) -> Result<usize, DomainError> {
    if raw_rows.is_empty() {
        return Ok(0);
    }

    let table_id = db
        .catalog()
        .get_table_id(table_name)
        .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

    // O(1) Zero-Allocation Fetch: Mengambil Arc<Schema> langsung dari CatalogStore
    let schema = db.catalog().get_schema(table_id)?;
    let columns = schema.columns();
    let total_rows = raw_rows.len();

    let table = db.get_table_storage_mut(table_name)?;

    // Identifikasi kolom berindeks untuk penyaringan alokasi entri indeks secara selektif
    let indexed_col_indices: Vec<(usize, ColumnId)> = columns
        .iter()
        .enumerate()
        .filter(|(_, col)| table.index_registry().has_index(col.id))
        .map(|(idx, col)| (idx, col.id))
        .collect();

    struct StagedRow {
        assigned_row_id: RowId,
        prepared_values: Vec<SqlValue>,
        index_entries: Vec<(ColumnId, SqlValue)>,
    }

    let mut staged_rows = Vec::with_capacity(total_rows);
    let mut staged_counters = table.auto_increment_counters().clone();
    let next_start_id = table.row_store().next_row_id();

    for (offset, mut row_values) in raw_rows.into_iter().enumerate() {
        if row_values.len() < columns.len() {
            row_values.resize(columns.len(), SqlValue::Null);
        }

        for (i, col) in columns.iter().enumerate() {
            let is_null = row_values[i].is_null();

            if col.is_auto_increment() && is_null {
                let counter = staged_counters
                    .get_mut(&col.id)
                    .expect("Counter auto-increment harus terinisialisasi");

                row_values[i] = SqlValue::Int(*counter);
                let step = match col.auto_increment_config() {
                    Some(AutoIncrement::Enabled { step, .. }) => *step,
                    _ => 1,
                };
                *counter += step;
            } else if is_null {
                if let Some(default_val) = col.default_value() {
                    row_values[i] = default_val.clone();
                }
            }
        }

        // Memanggil validate_row yang sudah dioptimasi dengan Fast-Path CHECK constraint
        schema.validate_row(&row_values)?;

        let index_entries: Vec<(ColumnId, SqlValue)> = indexed_col_indices
            .iter()
            .map(|&(c_idx, col_id)| (col_id, row_values[c_idx].clone()))
            .collect();

        let assigned_row_id = RowId::from(next_start_id + offset as u64);

        staged_rows.push(StagedRow {
            assigned_row_id,
            prepared_values: row_values,
            index_entries,
        });
    }

    // Komit transaksional ke Indeks B-Tree
    for (offset, staged) in staged_rows.iter().enumerate() {
        let entries_ref: Vec<(ColumnId, &SqlValue)> = staged
            .index_entries
            .iter()
            .map(|(col_id, val)| (*col_id, val))
            .collect();

        if let Err(err) = table
            .index_registry_mut()
            .insert_entry_ref(staged.assigned_row_id, &entries_ref)
        {
            // Rollback entri indeks jika terjadi kegagalan unik/kunci
            for rb_staged in staged_rows[..offset].iter() {
                let rb_entries_ref: Vec<(ColumnId, &SqlValue)> = rb_staged
                    .index_entries
                    .iter()
                    .map(|(col_id, val)| (*col_id, val))
                    .collect();

                let _ = table
                    .index_registry_mut()
                    .remove_entry_ref(rb_staged.assigned_row_id, &rb_entries_ref);
            }
            return Err(err);
        }
    }

    *table.auto_increment_counters_mut() = staged_counters;

    let rows_to_insert: Vec<Vec<SqlValue>> = staged_rows
        .into_iter()
        .map(|staged| staged.prepared_values)
        .collect();

    table.row_store_mut().insert_rows(rows_to_insert);

    Ok(total_rows)
}

pub(crate) fn handle_delete(
    db: &mut Database,
    table_name: &str,
    predicate: Option<&Expr>,
) -> Result<usize, DomainError> {
    let table_id = db
        .catalog()
        .get_table_id(table_name)
        .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

    let schema_cols = db
        .catalog()
        .get_schema_columns(table_id)
        .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

    let schema = Schema::new(schema_cols.to_vec())?;
    let columns = schema.columns();

    let table = db.get_table_storage_mut(table_name)?;

    let candidate_row_ids: Option<HashSet<RowId>> =
        try_index_scan(table, &schema, predicate).map(|ids| ids.into_iter().collect());

    let indexed_col_indices: Vec<(usize, ColumnId)> = columns
        .iter()
        .enumerate()
        .filter(|(_, col)| table.index_registry().has_index(col.id))
        .map(|(idx, col)| (idx, col.id))
        .collect();

    struct StagedDelete {
        row_idx: usize,
        row_id: RowId,
        index_entries: Vec<(ColumnId, SqlValue)>,
    }

    let mut staged_deletes = Vec::new();

    for (idx, row) in table.row_store().rows().iter().enumerate() {
        if let Some(ref valid_ids) = candidate_row_ids {
            if !valid_ids.contains(&row.id()) {
                continue;
            }
        }

        let matches_condition = match predicate {
            Some(expr) => eval_where(expr, row)?,
            None => true,
        };

        if matches_condition {
            let row_id = row.id();
            let index_entries: Vec<(ColumnId, SqlValue)> = indexed_col_indices
                .iter()
                .map(|&(c_idx, col_id)| (col_id, row.values()[c_idx].clone()))
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

    let mut removed_count = 0;

    for staged in &staged_deletes {
        let entries_ref: Vec<(ColumnId, &SqlValue)> = staged
            .index_entries
            .iter()
            .map(|(col_id, val)| (*col_id, val))
            .collect();

        if let Err(err) = table
            .index_registry_mut()
            .remove_entry_ref(staged.row_id, &entries_ref)
        {
            for rb_staged in staged_deletes[..removed_count].iter().rev() {
                let rb_entries_ref: Vec<(ColumnId, &SqlValue)> = rb_staged
                    .index_entries
                    .iter()
                    .map(|(col_id, val)| (*col_id, val))
                    .collect();

                let _ = table
                    .index_registry_mut()
                    .insert_entry_ref(rb_staged.row_id, &rb_entries_ref);
            }
            return Err(err);
        }
        removed_count += 1;
    }

    let deleted_count = staged_deletes.len();
    let indices_to_delete: Vec<usize> = staged_deletes.into_iter().map(|s| s.row_idx).collect();
    table
        .row_store_mut()
        .delete_rows_by_indices(indices_to_delete);

    Ok(deleted_count)
}

pub(crate) fn handle_update(
    db: &mut Database,
    table_name: &str,
    assignments: &HashMap<ColumnId, Expr>,
    predicate: Option<&Expr>,
) -> Result<usize, DomainError> {
    if assignments.is_empty() {
        return Ok(0);
    }

    let table_id = db
        .catalog()
        .get_table_id(table_name)
        .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

    let schema_cols = db
        .catalog()
        .get_schema_columns(table_id)
        .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

    let schema = Schema::new(schema_cols.to_vec())?;
    let columns = schema.columns();

    let table = db.get_table_storage_mut(table_name)?;

    let candidate_row_ids: Option<HashSet<RowId>> =
        try_index_scan(table, &schema, predicate).map(|ids| ids.into_iter().collect());

    let indexed_col_indices: Vec<(usize, ColumnId)> = columns
        .iter()
        .enumerate()
        .filter(|(_, col)| table.index_registry().has_index(col.id))
        .map(|(idx, col)| (idx, col.id))
        .collect();

    let mut staged_updates = Vec::new();

    for (idx, row) in table.row_store().rows().iter().enumerate() {
        if let Some(ref valid_ids) = candidate_row_ids {
            if !valid_ids.contains(&row.id()) {
                continue;
            }
        }

        let matches_condition = match predicate {
            Some(expr) => eval_where(expr, row)?,
            None => true,
        };

        if matches_condition {
            let row_id = row.id();
            let mut new_values = row.values().to_vec();
            let mut is_changed = false;

            for (col_idx, col) in columns.iter().enumerate() {
                if let Some(new_expr) = assignments.get(&col.id) {
                    let evaluated_cow = eval_expr(new_expr, row)?;
                    let evaluated_val = evaluated_cow.into_owned();

                    if evaluated_val != new_values[col_idx] {
                        new_values[col_idx] = evaluated_val;
                        is_changed = true;
                    }
                }
            }

            if !is_changed {
                continue;
            }

            schema.validate_row(&new_values)?;

            let old_entries: Vec<(ColumnId, SqlValue)> = indexed_col_indices
                .iter()
                .map(|&(c_idx, col_id)| (col_id, row.values()[c_idx].clone()))
                .collect();

            let new_entries: Vec<(ColumnId, SqlValue)> = indexed_col_indices
                .iter()
                .map(|&(c_idx, col_id)| (col_id, new_values[c_idx].clone()))
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

    let mut modified_count = 0;

    for staged in &staged_updates {
        let old_entries_ref: Vec<(ColumnId, &SqlValue)> = staged
            .old_entries
            .iter()
            .map(|(col_id, val)| (*col_id, val))
            .collect();

        let new_entries_ref: Vec<(ColumnId, &SqlValue)> = staged
            .new_entries
            .iter()
            .map(|(col_id, val)| (*col_id, val))
            .collect();

        if let Err(err) = table
            .index_registry_mut()
            .remove_entry_ref(staged.row_id, &old_entries_ref)
        {
            rollback_index_changes(table, &staged_updates[..modified_count]);
            return Err(err);
        }

        if let Err(err) = table
            .index_registry_mut()
            .insert_entry_ref(staged.row_id, &new_entries_ref)
        {
            let _ = table
                .index_registry_mut()
                .insert_entry_ref(staged.row_id, &old_entries_ref);
            rollback_index_changes(table, &staged_updates[..modified_count]);
            return Err(err);
        }

        modified_count += 1;
    }

    let updated_count = staged_updates.len();
    let updates: Vec<(usize, crate::Row)> = staged_updates
        .into_iter()
        .map(|staged| {
            (
                staged.row_idx,
                crate::Row::with_id(staged.row_id, staged.new_row_values),
            )
        })
        .collect();

    table.row_store_mut().update_rows_by_indices(updates);

    Ok(updated_count)
}

fn rollback_index_changes(table: &mut TableStorage, processed_updates: &[StagedUpdate]) {
    for staged in processed_updates.iter().rev() {
        let old_entries_ref: Vec<(ColumnId, &SqlValue)> = staged
            .old_entries
            .iter()
            .map(|(col_id, val)| (*col_id, val))
            .collect();

        let new_entries_ref: Vec<(ColumnId, &SqlValue)> = staged
            .new_entries
            .iter()
            .map(|(col_id, val)| (*col_id, val))
            .collect();

        let _ = table
            .index_registry_mut()
            .remove_entry_ref(staged.row_id, &new_entries_ref);
        let _ = table
            .index_registry_mut()
            .insert_entry_ref(staged.row_id, &old_entries_ref);
    }
}

pub(crate) fn try_index_scan(
    table: &TableStorage,
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
                    let index = table.index_registry().get_index(col_id)?;
                    return Some(index.lookup(val).to_vec());
                }
            }
            if let (Expr::Literal(val), Expr::Column(col_name)) = (left.as_ref(), right.as_ref()) {
                if let Some(col_id) = schema.get_column_by_name(col_name).map(|c| c.id) {
                    let index = table.index_registry().get_index(col_id)?;
                    return Some(index.lookup(val).to_vec());
                }
            }
            None
        }

        Expr::Binary { left, op, right } if *op == BinaryOp::Gt || *op == BinaryOp::GtEq => {
            if let (Expr::Column(col_name), Expr::Literal(val)) = (left.as_ref(), right.as_ref()) {
                if let Some(col_id) = schema.get_column_by_name(col_name).map(|c| c.id) {
                    let index = table.index_registry().get_index(col_id)?;
                    let min_bound = if *op == BinaryOp::Gt {
                        Bound::Excluded(val)
                    } else {
                        Bound::Included(val)
                    };
                    return Some(index.range_lookup(min_bound, Bound::Unbounded));
                }
            }
            None
        }

        Expr::Binary { left, op, right } if *op == BinaryOp::Lt || *op == BinaryOp::LtEq => {
            if let (Expr::Column(col_name), Expr::Literal(val)) = (left.as_ref(), right.as_ref()) {
                if let Some(col_id) = schema.get_column_by_name(col_name).map(|c| c.id) {
                    let index = table.index_registry().get_index(col_id)?;
                    let max_bound = if *op == BinaryOp::Lt {
                        Bound::Excluded(val)
                    } else {
                        Bound::Included(val)
                    };
                    return Some(index.range_lookup(Bound::Unbounded, max_bound));
                }
            }
            None
        }

        Expr::InList { expr, list } => {
            if let Expr::Column(col_name) = expr.as_ref() {
                if let Some(col_id) = schema.get_column_by_name(col_name).map(|c| c.id) {
                    let index = table.index_registry().get_index(col_id)?;
                    let mut matched_row_ids = HashSet::new();

                    for item in list {
                        if let Expr::Literal(val) = item {
                            for &row_id in index.lookup(val) {
                                matched_row_ids.insert(row_id);
                            }
                        } else {
                            return None;
                        }
                    }
                    return Some(matched_row_ids.into_iter().collect());
                }
            }
            None
        }

        _ => None,
    }
}
