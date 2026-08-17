use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::catalog::Metadata;
use crate::disk::{BufferPoolManager, TableHeap};
use crate::index::IndexRegistry;
use crate::validator::validate_enum_variants;
use crate::{
    Column, ColumnConstraint, ColumnPosition, DataType, DomainError, RowId, TableContext, TableId,
    ValueType,
};

// --- ALTER ACTION
pub(crate) fn apply_add_columns(
    meta: &mut Metadata,
    tables: &mut HashMap<TableId, TableContext>,
    table_name: &str,
    columns: Vec<(String, DataType, Vec<ColumnConstraint>, ColumnPosition)>,
) -> Result<(), DomainError> {
    if columns.is_empty() {
        return Ok(());
    }

    let table_id = meta.get_table_id(table_name)?;

    let mut seen = HashSet::new();
    // TAHAP VALIDASI: Cek semua kolom di batch terhadap katalog DAN duplikat internal
    for (col_name, _, _, _) in &columns {
        // Cek apakah sudah ada di database ATAU duplikat di dalam batch ini
        if meta.get_column_id(table_id, col_name).is_ok() || !seen.insert(col_name) {
            return Err(DomainError::ColumnAlreadyExists(col_name.clone().into()));
        }
    }

    for (col_name, sql_type, constraints, position) in columns {
        let current_schema = meta.get_schema(table_id)?;

        // OPTIMASI 1: Hybrid Lookup (Mencoba resolusi langsung dari skema, fallback ke pencarian case-insensitive)
        let target_idx = match position {
            ColumnPosition::First => 0,
            ColumnPosition::After(ref ref_col_name) => {
                let pos = current_schema
                    .get_column_index_by_name(ref_col_name)
                    .or_else(|| {
                        current_schema
                            .columns()
                            .iter()
                            .position(|c| c.name.eq_ignore_ascii_case(ref_col_name))
                    })
                    .ok_or_else(|| {
                        DomainError::eval_error(format!(
                            "Kolom referensi '{ref_col_name}' tidak ditemukan di skema"
                        ))
                    })?;
                pos + 1
            }
            ColumnPosition::Default => current_schema.columns().len(),
        };

        meta.register_column_at(
            table_id,
            &col_name,
            sql_type.clone(),
            constraints.clone(),
            target_idx,
        )?;

        let col_def = Column::with_constraints(
            col_id_from_register_dummy(),
            &col_name,
            sql_type,
            constraints,
        );
        // Catatan: Jika col_id dibutuhkan untuk default value, ambil dari register_column_at atau skema baru.
        let default_val = col_def.default_value().cloned().unwrap_or(ValueType::Null);

        let context = tables
            .get_mut(&table_id)
            .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

        // Modifikasi data fisik baris di halaman disk (TableHeap) secara page-oriented
        let rids = context
            .table_heap
            .scan_rids(&mut context.buffer_pool_manager)?;
        for rid in rids {
            if let Some(tuple_bytes) = context
                .table_heap
                .get_tuple(&mut context.buffer_pool_manager, rid)?
            {
                let mut row_values: Vec<ValueType> = bincode::deserialize(&tuple_bytes)
                    .map_err(|e| DomainError::storage(e.to_string()))?;

                // Sisipkan nilai default pada posisi target kolom baru
                if target_idx <= row_values.len() {
                    row_values.insert(target_idx, default_val.clone());
                } else {
                    row_values.push(default_val.clone());
                }

                // Perbarui tuple di disk
                context
                    .table_heap
                    .delete_tuple(&mut context.buffer_pool_manager, rid)?;
                let new_bytes = bincode::serialize(&row_values)
                    .map_err(|e| DomainError::storage(e.to_string()))?;
                context
                    .table_heap
                    .insert_tuple(&mut context.buffer_pool_manager, &new_bytes)?;
            }
        }
    }

    let new_schema = meta.get_schema(table_id)?;
    let context = tables
        .get_mut(&table_id)
        .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

    // OPTIMASI 2: Conditional Index Rebuild berbasis halaman fisik
    let rids = context
        .table_heap
        .scan_rids(&mut context.buffer_pool_manager)?;
    let has_rows = !rids.is_empty();
    let has_indexes = !context.index_registry.is_empty();

    if has_rows && has_indexes {
        rebuild_indexes_for_context(
            &mut context.table_heap,
            &mut context.buffer_pool_manager,
            &mut context.index_registry,
            &new_schema,
        )?;
    }

    Ok(())
}

pub(crate) fn apply_drop_column(
    meta: &mut Metadata,
    tables: &mut HashMap<TableId, TableContext>,
    table_name: &str,
    col_name: &str,
) -> Result<(), DomainError> {
    let table_id = meta.get_table_id(table_name)?;
    let col_id = meta.get_column_id(table_id, col_name)?;

    let current_schema = meta.get_schema(table_id)?;
    let col_idx = current_schema
        .get_column_index_by_name(col_name)
        .ok_or_else(|| DomainError::eval_error(format!("Kolom '{col_name}' tidak ditemukan")))?;

    meta.unregister_column(table_id, col_name)?;
    let new_schema = meta.get_schema(table_id)?;

    let context = tables
        .get_mut(&table_id)
        .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

    context.index_registry.drop_index(col_id);

    // Hapus kolom dari setiap baris di TableHeap
    let rids = context
        .table_heap
        .scan_rids(&mut context.buffer_pool_manager)?;
    for rid in rids {
        if let Some(tuple_bytes) = context
            .table_heap
            .get_tuple(&mut context.buffer_pool_manager, rid)?
        {
            let mut row_values: Vec<ValueType> = bincode::deserialize(&tuple_bytes)
                .map_err(|e| DomainError::storage(e.to_string()))?;

            if col_idx < row_values.len() {
                row_values.remove(col_idx);
            }

            context
                .table_heap
                .delete_tuple(&mut context.buffer_pool_manager, rid)?;
            let new_bytes =
                bincode::serialize(&row_values).map_err(|e| DomainError::storage(e.to_string()))?;
            context
                .table_heap
                .insert_tuple(&mut context.buffer_pool_manager, &new_bytes)?;
        }
    }

    let rids_after = context
        .table_heap
        .scan_rids(&mut context.buffer_pool_manager)?;
    let has_rows = !rids_after.is_empty();
    let has_indexes = !context.index_registry.is_empty();

    if has_rows && has_indexes {
        rebuild_indexes_for_context(
            &mut context.table_heap,
            &mut context.buffer_pool_manager,
            &mut context.index_registry, // Ubah dari 'context' menjadi ini
            &new_schema,
        )?;
    }

    Ok(())
}

pub(crate) fn apply_rename_column(
    meta: &mut Metadata,
    table_name: &str,
    old_name: &str,
    new_name: &str,
) -> Result<(), DomainError> {
    let table_id = meta.get_table_id(table_name)?;

    // cek apakah nama baru sudah dipakai di meta
    if meta.get_column_id(table_id, new_name).is_ok() {
        return Err(DomainError::eval_error(format!(
            "new name already axist: {}",
            new_name
        )));
    }
    let col_id = meta.get_column_id(table_id, old_name)?;

    meta.mutate_column(table_id, col_id, |col| {
        col.name = new_name.to_string();
    })?;

    // Karena rename hanya mengubah metadata katalog, tidak ada perubahan pada TableHeap/Index.
    Ok(())
}

pub(crate) fn apply_modify_column_type(
    meta: &mut Metadata,
    table_name: &str,
    col_name: &str,
    new_type: DataType,
) -> Result<(), DomainError> {
    validate_enum_variants(&new_type)?;

    let table_id = meta.get_table_id(table_name)?;
    let col_id = meta.get_column_id(table_id, col_name)?;

    meta.mutate_column(table_id, col_id, |col| {
        col.sql_type = new_type.clone();
    })?;

    Ok(())
}

pub(crate) fn apply_add_constraint(
    meta: &mut Metadata,
    tables: &mut HashMap<TableId, TableContext>,
    table_name: &str,
    col_name: &str,
    constraint: ColumnConstraint,
) -> Result<(), DomainError> {
    let table_id = meta.get_table_id(table_name)?;
    let col_id = meta.get_column_id(table_id, col_name)?;

    meta.mutate_column(table_id, col_id, |col| {
        if !col.constraints.contains(&constraint) {
            col.constraints.push(constraint.clone());
        }
    })?;

    let new_schema = meta.get_schema(table_id)?;

    if let Some(context) = tables.get_mut(&table_id) {
        if matches!(
            constraint,
            ColumnConstraint::Unique | ColumnConstraint::PrimaryKey
        ) {
            let _ = context.index_registry.create_btree_index(col_id, true);

            let rids = context
                .table_heap
                .scan_rids(&mut context.buffer_pool_manager)?;
            let has_rows = !rids.is_empty();
            let has_indexes = !context.index_registry.is_empty();

            if has_rows && has_indexes {
                rebuild_indexes_for_context(
                    &mut context.table_heap,
                    &mut context.buffer_pool_manager,
                    &mut context.index_registry, // Ubah dari 'context' menjadi ini
                    &new_schema,
                )?;
            }
        }
    }

    Ok(())
}

pub(crate) fn apply_drop_constraint(
    meta: &mut Metadata,
    table_name: &str,
    col_name: &str,
    constraint: ColumnConstraint,
) -> Result<(), DomainError> {
    let table_id = meta.get_table_id(table_name)?;
    let col_id = meta.get_column_id(table_id, col_name)?;

    meta.mutate_column(table_id, col_id, |col| {
        col.constraints.retain(|c| c != &constraint);
    })?;

    Ok(())
}

pub(crate) fn apply_set_default(
    meta: &mut Metadata,
    table_name: &str,
    col_name: &str,
    default_val: Option<ValueType>,
) -> Result<(), DomainError> {
    let table_id = meta.get_table_id(table_name)?;
    let col_id = meta.get_column_id(table_id, col_name)?;

    meta.mutate_column(table_id, col_id, |col| {
        col.constraints
            .retain(|c| !matches!(c, ColumnConstraint::Default(_)));
        col.constraints
            .push(ColumnConstraint::Default(ValueType::from(
                default_val.clone(),
            )));
    })?;

    Ok(())
}

// Helper lokal untuk membangun ulang indeks dari TableHeap
fn rebuild_indexes_for_context(
    table_heap: &mut TableHeap,
    bpm: &mut BufferPoolManager,
    index_registry: &mut IndexRegistry, // Cukup pinjam index_registry-nya saja
    schema: &crate::Schema,
) -> Result<(), DomainError> {
    index_registry.clear();
    let rids = table_heap.scan_rids(bpm)?;

    for rid in rids {
        if let Some(tuple_bytes) = table_heap.get_tuple(bpm, rid)? {
            let row_values: Vec<ValueType> = bincode::deserialize(&tuple_bytes)
                .map_err(|e| DomainError::storage(e.to_string()))?;

            let entries: Vec<(crate::ColumnId, &ValueType)> = schema
                .columns()
                .iter()
                .enumerate()
                .filter(|(_, col)| index_registry.has_index(col.id))
                .map(|(idx, col)| (col.id, &row_values[idx]))
                .collect();

            let row_id_alias = RowId::from(rid);
            index_registry.insert_entry_ref(row_id_alias, &entries)?;
        }
    }
    Ok(())
}

// Dummy helper jika diperlukan untuk mengambil col_id saat pembuatan Column objek sementara
fn col_id_from_register_dummy() -> crate::ColumnId {
    crate::ColumnId(0)
}
