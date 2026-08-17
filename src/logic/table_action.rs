use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::Arc,
};

use crate::{
    Column, ColumnConstraint, ColumnId, DataType, DomainError, Row, RowId, Schema, TableContext,
    TableId, ValueType,
    catalog::{Metadata, QueryResult},
    disk::{BufferPoolManager, DiskManager, TableHeap},
    index::IndexRegistry,
};

// --- TABLE ACTION
pub(crate) fn apply_create_table(
    catalog: &mut Metadata,
    tables: &mut HashMap<TableId, TableContext>,
    db_path: &str,
    table_name: &str,
    raw_columns: Vec<(String, DataType, Vec<ColumnConstraint>)>,
) -> Result<(), DomainError> {
    if raw_columns.is_empty() {
        return Err(DomainError::catalog(
            "Table tidak boleh kosong, setidaknya buat 1 kolom.",
        ));
    }

    // FASE 1: Validasi murni (tidak mengubah state sama sekali)
    let mut seen = HashSet::new();
    for (col_name, _, _) in &raw_columns {
        if !seen.insert(col_name) {
            return Err(DomainError::eval_error(format!(
                "duplicate columns batch {}",
                col_name,
            )));
        }
    }

    // FASE 2: Eksekusi setelah dijamin 100% aman
    let table_id = catalog.register_table(table_name)?;
    for (col_name, sql_type, constraints) in raw_columns {
        if let Err(err) = catalog.register_column(table_id, &col_name, sql_type, constraints) {
            let _ = catalog.unregister_table(table_name);
            return Err(err);
        }
    }
    let schema = catalog.get_schema(table_id)?;

    // Inisialisasi file fisik .db khusus untuk tabel baru
    let table_file_path = Path::new(db_path).join(format!("{}.db", table_name));
    let disk_manager = DiskManager::new(&table_file_path)?;
    let mut buffer_pool_manager = BufferPoolManager::new(disk_manager, 10);
    let table_heap = TableHeap::new(&mut buffer_pool_manager)?;

    let mut auto_increment_counters = HashMap::new();

    // --- TAMBAHAN: Inisialisasi IndexRegistry dan daftarkan kolom unik ---
    let mut index_registry = IndexRegistry::new(); //[span_4](start_span)[span_4](end_span)

    for col in schema.columns() {
        // 1. Cek konfigurasi auto increment
        if let Some(crate::schema::Increment::Enabled { start, .. }) = col.auto_increment_config() {
            auto_increment_counters.insert(col.id, *start);
        }

        // 2. Cek apakah kolom memiliki constraint Unique atau PrimaryKey[span_5](start_span)[span_5](end_span)[span_6](start_span)[span_6](end_span)
        let is_unique = col
            .constraints
            .iter()
            .any(|c| matches!(c, ColumnConstraint::Unique | ColumnConstraint::PrimaryKey));

        if is_unique {
            // Buat indeks B-Tree unik secara otomatis[span_7](start_span)[span_7](end_span)
            index_registry.create_btree_index(col.id, true)?; //[span_8](start_span)[span_8](end_span)
        }
    }
    // -------------------------------------------------------------------

    tables.insert(
        table_id,
        TableContext {
            table_heap,
            buffer_pool_manager,
            index_registry, // Masukkan index_registry yang sudah terisi
            auto_increment_counters,
        },
    );

    Ok(())
}

pub(crate) fn apply_drop_table(
    catalog: &mut Metadata,
    tables: &mut HashMap<TableId, TableContext>,
    db_path: &str,
    table_name: &str,
) -> Result<(), DomainError> {
    let table_id = catalog.unregister_table(table_name)?;
    tables.remove(&table_id);

    // Hapus file fisik .db milik tabel dari disk
    let table_file_path = Path::new(db_path).join(format!("{}.db", table_name));
    if table_file_path.exists() {
        std::fs::remove_file(&table_file_path).map_err(|e| DomainError::storage(e.to_string()))?;
    }

    if catalog.list_tables().is_empty() {}
    Ok(())
}

pub(crate) fn apply_rename_table(
    catalog: &mut Metadata,
    db_path: &str,
    old_name: &str,
    new_name: &str,
) -> Result<(), DomainError> {
    #[allow(warnings)]
    let table_id = catalog.rename_table(old_name, new_name)?;

    // Ubah nama file fisik .db di disk
    let old_path = Path::new(db_path).join(format!("{}.db", old_name));
    let new_path = Path::new(db_path).join(format!("{}.db", new_name));

    if old_path.exists() {
        std::fs::rename(&old_path, &new_path).map_err(|e| DomainError::storage(e.to_string()))?;
    }

    Ok(())
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

        // Menentukan apakah kolom boleh bernilai NULL menggunakan is_nullable()
        let null_str = if col.is_nullable() { "YES" } else { "NO" };

        // Menentukan status Primary Key menggunakan is_primary_key()
        let key_str = if col.is_primary_key() { "PRI" } else { "" };

        // Mengambil nilai default menggunakan default_value()
        let default_str = match col.default_value() {
            Some(val) => format!("{:?}", val),
            None => "NULL".to_string(),
        };

        // Menentukan informasi ekstra (seperti auto_increment) menggunakan is_auto_increment()
        let extra_str = if col.is_auto_increment() {
            "auto_increment"
        } else {
            ""
        };

        let values = vec![
            ValueType::Text(Arc::from(col.name.clone())), // Field
            ValueType::Text(Arc::from(format!("{:?}", col.sql_type))), // Type
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

pub(crate) fn virtual_column(list: Vec<String>) -> Result<(Schema, Vec<Row>), DomainError> {
    let col_def = Column::new(ColumnId(1), "table_name", DataType::Text);
    let schema = Schema::new(vec![col_def])?;

    let mut rows = Vec::with_capacity(list.len());

    for (idx, name) in list.into_iter().enumerate() {
        let row_id = RowId((idx + 1) as u64);
        let values = vec![ValueType::Text(Arc::from(name.as_str()))];
        rows.push(Row::with_id(row_id, values));
    }

    Ok((schema, rows))
}
