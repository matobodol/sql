use std::{collections::HashMap, path::Path, sync::Arc};

use crate::{
    BufferPoolManager, Column, ColumnConstraint, ColumnId, DataType, DiskManager, DomainError,
    QueryResult, Row, RowId, Schema, TableHeap, TableId, ValueType, catalog::CatalogStore,
    database::TableContext, index::IndexRegistry,
};

// --- TABLE ACTION
pub(crate) fn apply_create_table(
    catalog: &mut CatalogStore,
    tables: &mut HashMap<TableId, TableContext>,
    base_path: &str,
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

    // Inisialisasi file fisik .db khusus untuk tabel baru di dalam folder database terkait
    let table_file_path = Path::new(base_path).join(format!("{}.db", table_name));
    let disk_manager = DiskManager::new(&table_file_path)?;
    let mut buffer_pool_manager = BufferPoolManager::new(disk_manager, 10);
    let table_heap = TableHeap::new(&mut buffer_pool_manager)?;

    // --- TAMBAHAN: Inisialisasi auto_increment_counters dari Schema tabel baru ---
    let schema = catalog.get_schema(table_id)?;
    let mut auto_increment_counters = HashMap::new();

    for col in schema.columns() {
        if let Some(crate::schema::Increment::Enabled { start, .. }) = col.auto_increment_config() {
            auto_increment_counters.insert(col.id, *start);
        }
    }
    // -------------------------------------------------------------------------

    tables.insert(
        table_id,
        TableContext {
            table_heap,
            buffer_pool_manager,
            index_registry: IndexRegistry::new(),
            auto_increment_counters, // <-- Masukkan ke TableContext
        },
    );

    Ok(())
}

pub(crate) fn apply_drop_table(
    catalog: &mut CatalogStore,
    tables: &mut HashMap<TableId, TableContext>,
    base_path: &str,
    table_name: &str,
) -> Result<(), DomainError> {
    let table_id = catalog.unregister_table(table_name)?;
    tables.remove(&table_id);

    // Hapus file fisik .db milik tabel dari disk
    let table_file_path = Path::new(base_path).join(format!("{}.db", table_name));
    if table_file_path.exists() {
        std::fs::remove_file(&table_file_path).map_err(|e| DomainError::storage(e.to_string()))?;
    }

    if catalog.list_tables().is_empty() {}
    Ok(())
}

pub(crate) fn apply_rename_table(
    catalog: &mut CatalogStore,
    base_path: &str,
    old_name: &str,
    new_name: &str,
) -> Result<(), DomainError> {
    #[allow(warnings)]
    let table_id = catalog.rename_table(old_name, new_name)?;

    // Ubah nama file fisik .db di disk
    let old_path = Path::new(base_path).join(format!("{}.db", old_name));
    let new_path = Path::new(base_path).join(format!("{}.db", new_name));

    if old_path.exists() {
        std::fs::rename(&old_path, &new_path).map_err(|e| DomainError::storage(e.to_string()))?;
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

    for (idx, col) in columns.iter().enumerate() {
        let row_id = RowId((idx + 1) as u64);

        // Menentukan apakah kolom boleh bernilai NULL menggunakan is_nullable()[span_1](start_span)[span_1](end_span)
        let null_str = if col.is_nullable() { "YES" } else { "NO" };

        // Menentukan status Primary Key menggunakan is_primary_key()[span_2](start_span)[span_2](end_span)
        let key_str = if col.is_primary_key() { "PRI" } else { "" };

        // Mengambil nilai default menggunakan default_value()[span_3](start_span)[span_3](end_span)
        let default_str = match col.default_value() {
            Some(val) => format!("{:?}", val),
            None => "NULL".to_string(),
        };

        // Menentukan informasi ekstra (seperti auto_increment) menggunakan is_auto_increment()[span_4](start_span)[span_4](end_span)
        let extra_str = if col.is_auto_increment() {
            "auto_increment"
        } else {
            ""
        };

        let values = vec![
            ValueType::Text(Arc::from(col.name.clone())), // Field[span_5](start_span)[span_5](end_span)
            ValueType::Text(Arc::from(format!("{:?}", col.sql_type))), // Type[span_6](start_span)[span_6](end_span)
            ValueType::Text(Arc::from(null_str)),
            ValueType::Text(Arc::from(key_str)),
            ValueType::Text(Arc::from(default_str)),
            ValueType::Text(Arc::from(extra_str)),
        ];

        rows.push(Row::with_id(row_id, values));
    }

    Ok(QueryResult::Dql {
        schema: desc_schema,
        rows,
    })
}
