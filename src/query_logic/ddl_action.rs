use std::collections::HashMap;
use std::sync::Arc;

use crate::catalog::CatalogStore;
use crate::command::ColumnPosition;
use crate::id::{ColumnId, TableId};
use crate::table_store::TableStorage;
use crate::{Column, ColumnConstraint, DomainError, Schema, SqlType, SqlValue};

// --- PRIVATE HANDLER FUNCTIONS ---

pub(crate) fn create_table(
    catalog: &mut CatalogStore,
    tables: &mut HashMap<TableId, TableStorage>,
    table_name: &str,
    raw_columns: Vec<(String, SqlType, Vec<ColumnConstraint>)>,
) -> Result<TableId, DomainError> {
    let table_id = catalog.register_table(table_name)?;

    for (col_name, sql_type, constraints) in raw_columns {
        if let Err(err) = catalog.register_column(table_id, &col_name, sql_type, constraints) {
            let _ = catalog.unregister_table(table_name);
            return Err(err);
        }
    }

    let schema_cols = catalog.get_schema_columns(table_id).ok_or_else(|| {
        DomainError::TableNotFound(Arc::from(format!(
            "Gagal mengambil skema untuk tabel '{table_name}'"
        )))
    })?;

    if let Err(err) = Schema::validate_schema_columns(&schema_cols) {
        let _ = catalog.unregister_table(table_name);
        return Err(err);
    }

    let table_storage = TableStorage::new_with_arc(table_id, table_name, schema_cols);
    tables.insert(table_id, table_storage);

    Ok(table_id)
}

pub(crate) fn drop_table(
    catalog: &mut CatalogStore,
    tables: &mut HashMap<TableId, TableStorage>,
    table_name: &str,
) -> Result<(), DomainError> {
    let table_id = catalog.unregister_table(table_name)?;
    tables.remove(&table_id);
    Ok(())
}

pub(crate) fn execute_add_columns(
    catalog: &mut CatalogStore,
    tables: &mut HashMap<TableId, TableStorage>,
    table_name: &str,
    columns: Vec<(String, SqlType, Vec<ColumnConstraint>, ColumnPosition)>,
) -> Result<(), DomainError> {
    if columns.is_empty() {
        return Ok(());
    }

    let table_id = catalog
        .get_table_id(table_name)
        .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

    let schema_arc = catalog
        .get_schema_columns(table_id)
        .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;
    let mut staged_columns = (*schema_arc).to_vec();

    let mut insertions = Vec::with_capacity(columns.len());

    for (col_name, sql_type, constraints, position) in columns {
        let target_idx = match position {
            ColumnPosition::First => 0,
            ColumnPosition::After(ref ref_col_name) => {
                let pos = staged_columns
                    .iter()
                    .position(|c| c.name.eq_ignore_ascii_case(ref_col_name))
                    .ok_or_else(|| {
                        DomainError::eval_error(format!(
                            "Kolom referensi '{ref_col_name}' tidak ditemukan di skema"
                        ))
                    })?;
                pos + 1
            }
            ColumnPosition::Default => staged_columns.len(),
        };

        let temp_id = ColumnId(u32::MAX - staged_columns.len() as u32);
        let dummy_col_def =
            Column::with_constraints(temp_id, &col_name, sql_type.clone(), constraints.clone());

        staged_columns.insert(target_idx, dummy_col_def);
        insertions.push((target_idx, col_name, sql_type, constraints));
    }

    Schema::validate_schema_columns(&staged_columns)?;

    let mut committed_insertions = Vec::with_capacity(insertions.len());
    for (target_idx, col_name, sql_type, constraints) in insertions {
        let col_id =
            catalog.register_column(table_id, &col_name, sql_type.clone(), constraints.clone())?;
        let real_col_def = Column::with_constraints(col_id, &col_name, sql_type, constraints);

        let default_val = real_col_def
            .default_value()
            .cloned()
            .unwrap_or(SqlValue::Null);

        committed_insertions.push((target_idx, default_val));
    }

    let final_schema_cols = catalog
        .get_schema_columns(table_id)
        .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

    let table_storage = tables
        .get_mut(&table_id)
        .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

    for (target_idx, default_val) in committed_insertions {
        table_storage
            .row_store_mut()
            .add_column_to_rows(target_idx, default_val);
    }

    table_storage.rebuild_indexes(&final_schema_cols)?;

    Ok(())
}

pub(crate) fn execute_drop_column(
    catalog: &mut CatalogStore,
    tables: &mut HashMap<TableId, TableStorage>,
    table_name: &str,
    col_name: &str,
) -> Result<(), DomainError> {
    let table_id = catalog
        .get_table_id(table_name)
        .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

    let col_id = catalog
        .get_column_id(table_id, col_name)
        .ok_or_else(|| DomainError::eval_error(format!("Kolom '{col_name}' tidak ditemukan")))?;

    catalog.unregister_column(table_id, col_name)?;

    let new_schema_cols = catalog
        .get_schema_columns(table_id)
        .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

    let table_storage = tables
        .get_mut(&table_id)
        .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

    table_storage.index_registry_mut().drop_index(col_id);
    table_storage.rebuild_indexes(&new_schema_cols)?;

    Ok(())
}

pub(crate) fn execute_rename_column(
    catalog: &mut CatalogStore,
    table_name: &str,
    old_name: &str,
    new_name: &str,
) -> Result<(), DomainError> {
    let table_id = catalog
        .get_table_id(table_name)
        .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

    let col_id = catalog
        .get_column_id(table_id, old_name)
        .ok_or_else(|| DomainError::eval_error(format!("Kolom '{old_name}' tidak ditemukan")))?;

    catalog.mutate_column(table_id, col_id, |col| {
        col.name = new_name.to_string();
    })
}

pub(crate) fn execute_rename_table(
    catalog: &mut CatalogStore,
    tables: &mut HashMap<TableId, TableStorage>,
    old_name: &str,
    new_name: &str,
) -> Result<(), DomainError> {
    let table_id = catalog.rename_table(old_name, new_name)?;

    if let Some(table_storage) = tables.get_mut(&table_id) {
        table_storage.set_name(new_name);
    }

    Ok(())
}

pub(crate) fn execute_modify_column_type(
    catalog: &mut CatalogStore,
    table_name: &str,
    col_name: &str,
    new_type: SqlType,
) -> Result<(), DomainError> {
    new_type.validate_enum_variants()?;

    let table_id = catalog
        .get_table_id(table_name)
        .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

    let col_id = catalog
        .get_column_id(table_id, col_name)
        .ok_or_else(|| DomainError::eval_error(format!("Kolom '{col_name}' tidak ditemukan")))?;

    catalog.mutate_column(table_id, col_id, |col| {
        col.sql_type = new_type;
    })
}

pub(crate) fn execute_add_constraint(
    catalog: &mut CatalogStore,
    tables: &mut HashMap<TableId, TableStorage>,
    table_name: &str,
    col_name: &str,
    constraint: ColumnConstraint,
) -> Result<(), DomainError> {
    let table_id = catalog
        .get_table_id(table_name)
        .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

    let col_id = catalog
        .get_column_id(table_id, col_name)
        .ok_or_else(|| DomainError::eval_error(format!("Kolom '{col_name}' tidak ditemukan")))?;

    catalog.mutate_column(table_id, col_id, |col| {
        if !col.constraints.contains(&constraint) {
            col.constraints.push(constraint.clone());
        }
    })?;

    if matches!(
        constraint,
        ColumnConstraint::Unique | ColumnConstraint::PrimaryKey
    ) {
        let schema_cols = catalog
            .get_schema_columns(table_id)
            .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

        if let Some(table_storage) = tables.get_mut(&table_id) {
            let _ = table_storage
                .index_registry_mut()
                .create_btree_index(col_id, true);
            table_storage.rebuild_indexes(&schema_cols)?;
        }
    }

    Ok(())
}

pub(crate) fn execute_drop_constraint(
    catalog: &mut CatalogStore,
    table_name: &str,
    col_name: &str,
    constraint: &ColumnConstraint,
) -> Result<(), DomainError> {
    let table_id = catalog
        .get_table_id(table_name)
        .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

    let col_id = catalog
        .get_column_id(table_id, col_name)
        .ok_or_else(|| DomainError::eval_error(format!("Kolom '{col_name}' tidak ditemukan")))?;

    catalog.mutate_column(table_id, col_id, |col| {
        col.constraints.retain(|c| c != constraint);
    })
}

pub(crate) fn execute_set_default(
    catalog: &mut CatalogStore,
    table_name: &str,
    col_name: &str,
    default_val: Option<SqlValue>,
) -> Result<(), DomainError> {
    let table_id = catalog
        .get_table_id(table_name)
        .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

    let col_id = catalog
        .get_column_id(table_id, col_name)
        .ok_or_else(|| DomainError::eval_error(format!("Kolom '{col_name}' tidak ditemukan")))?;

    catalog.mutate_column(table_id, col_id, |col| {
        col.constraints
            .retain(|c| !matches!(c, ColumnConstraint::Default(_)));
        if let Some(val) = default_val {
            col.constraints.push(ColumnConstraint::Default(val));
        }
    })
}
