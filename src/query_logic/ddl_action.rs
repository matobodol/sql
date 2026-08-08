use std::collections::HashMap;
use std::sync::Arc;

use crate::catalog::catalog_store::CatalogStore;
use crate::schema::Column;
use crate::schema::column_constraint::ColumnConstraint;
use crate::storage::table_store::TableStorage;
use crate::types::sql_type::SqlType;
use crate::types::sql_value::SqlValue;
use crate::validator::validate_enum_variants;
use crate::{ColumnPosition, DomainError, QueryResult, TableId};

pub(crate) fn apply_create_table(
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

    let schema = catalog.get_schema(table_id)?;
    let table_storage = TableStorage::new(table_id, table_name, schema);
    tables.insert(table_id, table_storage);

    Ok(table_id)
}

pub(crate) fn apply_drop_table(
    catalog: &mut CatalogStore,
    tables: &mut HashMap<TableId, TableStorage>,
    table_name: &str,
) -> Result<QueryResult, DomainError> {
    let table_id = catalog.unregister_table(table_name)?;
    tables.remove(&table_id);
    Ok(QueryResult::OK)
}

pub(crate) fn apply_add_columns(
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

    for (col_name, sql_type, constraints, position) in columns {
        let current_schema = catalog.get_schema(table_id)?;

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

        let col_id = catalog.register_column_at(
            table_id,
            &col_name,
            sql_type.clone(),
            constraints.clone(),
            target_idx,
        )?;

        let col_def = Column::with_constraints(col_id, &col_name, sql_type, constraints);
        let default_val = col_def.default_value().cloned().unwrap_or(SqlValue::Null);

        let table_storage = tables
            .get_mut(&table_id)
            .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

        table_storage
            .row_store_mut()
            .add_column_to_rows(target_idx, default_val);
    }

    let new_schema = catalog.get_schema(table_id)?;
    let table_storage = tables
        .get_mut(&table_id)
        .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

    table_storage.update_schema(new_schema.clone());

    // OPTIMASI 2: Conditional Index Rebuild (Hanya rebuild jika tabel memiliki baris DAN memiliki indeks aktif)
    let has_rows = !table_storage.row_store().rows().is_empty();
    let has_indexes = !table_storage.index_registry().is_empty();

    if has_rows && has_indexes {
        table_storage.rebuild_indexes(&new_schema)?;
    }

    Ok(())
}

pub(crate) fn apply_drop_column(
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
    let new_schema = catalog.get_schema(table_id)?;

    let table_storage = tables
        .get_mut(&table_id)
        .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

    table_storage.update_schema(new_schema.clone());
    table_storage.index_registry_mut().drop_index(col_id);

    // Penerapan Conditional Index Rebuild yang serupa pada penghapusan kolom
    let has_rows = !table_storage.row_store().rows().is_empty();
    let has_indexes = !table_storage.index_registry().is_empty();

    if has_rows && has_indexes {
        table_storage.rebuild_indexes(&new_schema)?;
    }

    Ok(())
}

pub(crate) fn apply_rename_column(
    catalog: &mut CatalogStore,
    tables: &mut HashMap<TableId, TableStorage>,
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
    })?;

    let new_schema = catalog.get_schema(table_id)?;
    if let Some(table_storage) = tables.get_mut(&table_id) {
        table_storage.update_schema(new_schema);
    }

    Ok(())
}

pub(crate) fn apply_rename_table(
    catalog: &mut CatalogStore,
    tables: &mut HashMap<TableId, TableStorage>,
    old_name: &str,
    new_name: &str,
) -> Result<QueryResult, DomainError> {
    let table_id = catalog.rename_table(old_name, new_name)?;

    if let Some(table_storage) = tables.get_mut(&table_id) {
        table_storage.set_name(new_name);
    }

    Ok(QueryResult::OK)
}

pub(crate) fn apply_modify_column_type(
    catalog: &mut CatalogStore,
    tables: &mut HashMap<TableId, TableStorage>,
    table_name: &str,
    col_name: &str,
    new_type: SqlType,
) -> Result<(), DomainError> {
    validate_enum_variants(&new_type)?;

    let table_id = catalog
        .get_table_id(table_name)
        .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

    let col_id = catalog
        .get_column_id(table_id, col_name)
        .ok_or_else(|| DomainError::eval_error(format!("Kolom '{col_name}' tidak ditemukan")))?;

    catalog.mutate_column(table_id, col_id, |col| {
        col.sql_type = new_type.clone();
    })?;

    let new_schema = catalog.get_schema(table_id)?;
    if let Some(table_storage) = tables.get_mut(&table_id) {
        table_storage.update_schema(new_schema);
    }

    Ok(())
}

pub(crate) fn apply_add_constraint(
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

    let new_schema = catalog.get_schema(table_id)?;

    if let Some(table_storage) = tables.get_mut(&table_id) {
        table_storage.update_schema(new_schema.clone());

        if matches!(
            constraint,
            ColumnConstraint::Unique | ColumnConstraint::PrimaryKey
        ) {
            let _ = table_storage
                .index_registry_mut()
                .create_btree_index(col_id, true);

            let has_rows = !table_storage.row_store().rows().is_empty();
            let has_indexes = !table_storage.index_registry().is_empty();
            if has_rows && has_indexes {
                table_storage.rebuild_indexes(&new_schema)?;
            }
        }
    }

    Ok(())
}

pub(crate) fn apply_drop_constraint(
    catalog: &mut CatalogStore,
    tables: &mut HashMap<TableId, TableStorage>,
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
    })?;

    let new_schema = catalog.get_schema(table_id)?;
    if let Some(table_storage) = tables.get_mut(&table_id) {
        table_storage.update_schema(new_schema);
    }

    Ok(())
}

pub(crate) fn apply_set_default(
    catalog: &mut CatalogStore,
    tables: &mut HashMap<TableId, TableStorage>,
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
        col.constraints
            .push(ColumnConstraint::Default(SqlValue::from(
                default_val.clone(),
            )));
    })?;

    let new_schema = catalog.get_schema(table_id)?;
    if let Some(table_storage) = tables.get_mut(&table_id) {
        table_storage.update_schema(new_schema);
    }

    Ok(())
}
