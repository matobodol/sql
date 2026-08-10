use std::{collections::HashMap, sync::Arc};

use crate::{
    Column, ColumnConstraint, ColumnId, DataType, DomainError, QueryResult, Row, RowId, Schema,
    TableId, TableStorage, ValueType, catalog::CatalogStore,
};

// --- TABLE ACTION
pub(crate) fn apply_create_table(
    catalog: &mut CatalogStore,
    tables: &mut HashMap<TableId, TableStorage>,
    table_name: &str,
    raw_columns: Vec<(String, DataType, Vec<ColumnConstraint>)>,
) -> Result<(), DomainError> {
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

    Ok(())
}

pub(crate) fn apply_drop_table(
    catalog: &mut CatalogStore,
    tables: &mut HashMap<TableId, TableStorage>,
    table_name: &str,
) -> Result<(), DomainError> {
    let table_id = catalog.unregister_table(table_name)?;
    tables.remove(&table_id);
    if catalog.list_tables().is_empty() {}
    Ok(())
}

pub(crate) fn apply_rename_table(
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

pub(crate) fn execute_show_tables(list_tables: &[String]) -> Result<QueryResult, DomainError> {
    let col_def = Column::new(ColumnId(1), "table_name", DataType::Text);
    let schema = Schema::new(vec![col_def])?;

    let mut rows = Vec::with_capacity(list_tables.len());

    for (idx, name) in list_tables.into_iter().enumerate() {
        let row_id = RowId((idx + 1) as u64);
        let values = vec![ValueType::Text(Arc::from(name.as_str()))];
        rows.push(Row::with_id(row_id, values));
    }

    Ok(QueryResult::Dql { schema, rows })
}

pub(crate) fn execute_describe_table(columns: &[Column]) -> Result<QueryResult, DomainError> {
    let desc_schema = Schema::new(vec![
        Column::new(ColumnId(1), "Field", DataType::Text),
        Column::new(ColumnId(2), "Type", DataType::Text),
        Column::new(ColumnId(3), "Null", DataType::Text),
        Column::new(ColumnId(4), "Key", DataType::Text),
        Column::new(ColumnId(5), "Default", DataType::Text),
        Column::new(ColumnId(6), "Extra", DataType::Text),
    ])?;

    let mut rows = Vec::with_capacity(columns.len());

    // Iterasi kolom dan gunakan method helper dari struct Column
    for (idx, col) in columns.iter().enumerate() {
        let row_id = RowId((idx + 1) as u64);

        // Menentukan apakah kolom boleh bernilai NULL menggunakan is_nullable()[span_5](start_span)[span_5](end_span)
        let null_str = if col.is_nullable() { "YES" } else { "NO" };

        // Menentukan status Primary Key menggunakan is_primary_key()[span_6](start_span)[span_6](end_span)
        let key_str = if col.is_primary_key() { "PRI" } else { "" };

        // Mengambil nilai default menggunakan default_value()[span_7](start_span)[span_7](end_span)
        let default_str = match col.default_value() {
            Some(val) => format!("{:?}", val),
            None => "NULL".to_string(),
        };

        // Menentukan informasi ekstra (seperti auto_increment) menggunakan is_auto_increment()[span_8](start_span)[span_8](end_span)
        let extra_str = if col.is_auto_increment() {
            "auto_increment"
        } else {
            ""
        };

        let values = vec![
            ValueType::Text(Arc::from(col.name.clone())), // Field (menggunakan col.name)
            ValueType::Text(Arc::from(format!("{:?}", col.sql_type))), // Type (menggunakan col.sql_type)
            ValueType::Text(Arc::from(null_str)),                      // Null
            ValueType::Text(Arc::from(key_str)),                       // Key
            ValueType::Text(Arc::from(default_str)),                   // Default
            ValueType::Text(Arc::from(extra_str)),                     // Extra
        ];

        rows.push(Row::with_id(row_id, values));
    }

    Ok(QueryResult::Dql {
        schema: desc_schema,
        rows,
    })
}
