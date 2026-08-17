use std::sync::Arc;
use std::{collections::HashMap, path::Path};

use crate::catalog::{BASE_PATH, EXT_AUTO_INC, EXT_INDEX_REGISTRY, METADATA};
use crate::disk::{BufferPoolManager, DiskManager, TableHeap};
use crate::expression::evaluator::bind_expr;
use crate::index::IndexRegistry;
use crate::logic::dql_action::Statement;
use crate::logic::table_action::virtual_column;
use crate::logic::{
    apply_add_columns, apply_add_constraint, apply_create_table, apply_drop_column,
    apply_drop_constraint, apply_drop_table, apply_modify_column_type, apply_rename_column,
    apply_rename_table, apply_set_default, handle_delete, handle_insert, handle_update,
};
use crate::table_store::DiskStorage;
use crate::{
    ColumnConstraint, ColumnPosition, DataType, Expr, Row, Schema, TableContext, ValueType,
};
use crate::{
    DomainError, TableId,
    catalog::Metadata,
    logic::{execute_describe_table, execute_select},
};

#[derive(Debug, Clone, PartialEq)]
pub enum QueryResult {
    Inserted(usize),
    Updated(usize),
    Deleted(usize),
    Dql { schema: Schema, rows: Vec<Row> },
    OK,
}

#[derive(Debug)]
pub struct Database {
    metadata: Metadata,
    // Memetakan TableId ke konteks tabel (menyimpan data di file .db terpisah)
    tables: HashMap<TableId, TableContext>,
    db_path: String,
}

impl Database {
    /// Membuat instance Database baru dengan path direktori fisik tertentu
    pub fn new(username: &str, dbname: &str) -> Self {
        let db_path = format!(
            "{}/{}/{}",
            BASE_PATH,
            username.to_lowercase(),
            dbname.to_lowercase()
        );

        // Buat folder fisik untuk database secara otomatis saat diinisialisasi
        let _ = std::fs::create_dir_all(&db_path);

        Self {
            metadata: Metadata::new(),
            tables: HashMap::new(),
            db_path,
        }
    }

    /// Menyimpan metadata ke metadata.bin dan meng-flush semua halaman tabel ke file .db masing-masing
    pub fn save_to_disk(&mut self) -> Result<(), DomainError> {
        let path = Path::new(&self.db_path);
        std::fs::create_dir_all(path).map_err(|e| DomainError::storage(e.to_string()))?;

        // 1. Simpan metadata katalog (`metadata.bin`)
        let metadata_path = path.join(METADATA);
        DiskStorage::save_to_file(&metadata_path, &self.metadata)?;

        // 2. Flush buffer pool, simpan index, dan simpan auto_increment_counters
        for (table_id, context) in &mut self.tables {
            let table_name = self.metadata.get_table_name(*table_id)?;
            context.buffer_pool_manager.flush_all_pages()?;

            // Simpan indeks B-Tree tabel ke file terpisah
            let index_path = path.join(format!("{}{}", table_name, EXT_INDEX_REGISTRY));
            DiskStorage::save_to_file(&index_path, &context.index_registry)?;

            // --- TAMBAHAN: Simpan state auto_increment_counters ---
            let auto_inc_path = path.join(format!("{}{}", table_name, EXT_AUTO_INC));
            DiskStorage::save_to_file(&auto_inc_path, &context.auto_increment_counters)?;
            // -----------------------------------------------------
        }

        Ok(())
    }

    /// Memuat database dari struktur folder disk yang diminta
    pub fn load_from_disk(username: &str, dbname: &str) -> Result<Self, DomainError> {
        let db_path = format!(
            "{}/{}/{}",
            BASE_PATH,
            username.to_lowercase(),
            dbname.to_lowercase()
        );
        let path = Path::new(&db_path);

        // 1. Load metadata katalog (`metadata.bin`)
        let metadata_path = path.join(METADATA);
        let meta: Metadata = DiskStorage::load_from_file(&metadata_path)?;

        let mut tables = HashMap::new();

        // 2. Load setiap tabel fisik (`users.db`, `karyawan.db`, dll.)
        for table_name in meta.list_tables() {
            let table_id = meta.get_table_id(&table_name)?;
            let table_file_path = path.join(format!("{}.db", table_name));

            if table_file_path.exists() {
                // Inisialisasi DiskManager dan BufferPoolManager untuk file tabel tersebut
                let disk_manager = DiskManager::new(&table_file_path)?;
                let buffer_pool_manager = BufferPoolManager::new(disk_manager, 10);

                // Asumsikan halaman pertama adalah awal dari TableHeap
                let table_heap = TableHeap::from_first_page(0);

                let index_path = path.join(format!("{}{}", table_name, EXT_INDEX_REGISTRY));
                let index_registry = if index_path.exists() {
                    DiskStorage::load_from_file(&index_path)?
                } else {
                    IndexRegistry::new()
                };

                let auto_inc_path = path.join(format!("{}{}", table_name, EXT_AUTO_INC));
                let auto_increment_counters = if auto_inc_path.exists() {
                    DiskStorage::load_from_file(&auto_inc_path)?
                } else {
                    let schema = meta.get_schema(table_id)?;
                    let mut counters = HashMap::new();
                    for col in schema.columns() {
                        if let Some(crate::schema::Increment::Enabled { start, .. }) =
                            col.auto_increment_config()
                        {
                            counters.insert(col.id, *start);
                        }
                    }
                    counters
                };

                tables.insert(
                    table_id,
                    TableContext {
                        table_heap,
                        buffer_pool_manager,
                        index_registry,
                        auto_increment_counters,
                    },
                );
            }
        }

        Ok(Self {
            metadata: meta,
            tables,
            db_path,
        })
    }
}

impl Database {
    // ==========================================
    // GETTERS & HELPERS
    // ==========================================

    /// Mengambil referensi ke katalog metadata.
    pub fn meta(&self) -> &Metadata {
        &self.metadata
    }

    /// Mengambil referensi mutabel ke katalog metadata.
    pub fn meta_mut(&mut self) -> &mut Metadata {
        &mut self.metadata
    }

    /// Mengambil referensi ke peta penyimpanan tabel fisik (`TableContext`).
    pub fn tables(&self) -> &HashMap<TableId, TableContext> {
        &self.tables
    }

    /// Mendapatkan akses mutabel ke konteks tabel spesifik berdasarkan ID-nya.
    pub fn table_context_mut(&mut self, table_id: TableId) -> Option<&mut TableContext> {
        self.tables.get_mut(&table_id)
    }
    pub fn db_path(&self) -> &String {
        &self.db_path
    }
}

impl Database {
    // -- TABLE ACTION
    pub fn create_table(
        &mut self,
        table_name: &str,
        raw_columns: Vec<(String, crate::DataType, Vec<crate::ColumnConstraint>)>,
    ) -> Result<(), DomainError> {
        apply_create_table(
            &mut self.metadata,
            &mut self.tables,
            &self.db_path,
            table_name,
            raw_columns,
        )?;
        Ok(())
    }
    pub fn rename_table(&mut self, old_name: &str, new_name: &str) -> Result<(), DomainError> {
        apply_rename_table(&mut self.metadata, &self.db_path, old_name, new_name)
    }
    pub fn drop_table(&mut self, table_name: &str) -> Result<(), DomainError> {
        apply_drop_table(
            &mut self.metadata,
            &mut self.tables,
            &self.db_path,
            table_name,
        )
    }
    pub fn show_tables(&self) -> Result<(Schema, Vec<Row>), DomainError> {
        let table_names = self.metadata.list_tables();
        virtual_column(table_names)
    }
    pub fn describe_table(&self, table_name: &str) -> Result<QueryResult, DomainError> {
        let table_id = self.metadata.get_table_id(&table_name)?;
        let schema = self.metadata.get_schema(table_id)?;
        execute_describe_table(schema.columns())
    }

    // -- DDL ACTION
    pub fn add_columns(
        &mut self,
        table_name: &str,
        raw_columns: Vec<(String, DataType, Vec<ColumnConstraint>, ColumnPosition)>,
    ) -> Result<(), DomainError> {
        apply_add_columns(
            &mut self.metadata,
            &mut self.tables,
            table_name,
            raw_columns,
        )
    }
    pub fn drop_column(&mut self, table_name: &str, col_name: &str) -> Result<(), DomainError> {
        apply_drop_column(&mut self.metadata, &mut self.tables, table_name, col_name)
    }
    pub fn rename_column(
        &mut self,
        table_name: &str,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), DomainError> {
        apply_rename_column(&mut self.metadata, table_name, old_name, new_name)
    }
    pub fn modify_column_type(
        &mut self,
        table_name: &str,
        col_name: &str,
        new_type: DataType,
    ) -> Result<(), DomainError> {
        apply_modify_column_type(&mut self.metadata, table_name, col_name, new_type)
    }
    pub fn add_column_constraint(
        &mut self,
        table_name: &str,
        col_name: &str,
        constraint: ColumnConstraint,
    ) -> Result<(), DomainError> {
        apply_add_constraint(
            &mut self.metadata,
            &mut self.tables,
            table_name,
            col_name,
            constraint,
        )
    }
    pub fn drop_column_constraint(
        &mut self,
        table_name: &str,
        col_name: &str,
        constraint: ColumnConstraint,
    ) -> Result<(), DomainError> {
        apply_drop_constraint(&mut self.metadata, table_name, col_name, constraint)
    }
    pub fn set_default(
        &mut self,
        table_name: &str,
        col_name: &str,
        default_val: Option<ValueType>,
    ) -> Result<(), DomainError> {
        apply_set_default(&mut self.metadata, table_name, col_name, default_val)
    }
    // -- DML ACTION
    pub fn insert(
        &mut self,
        table_name: &str,
        rows: Vec<Vec<ValueType>>,
    ) -> Result<QueryResult, DomainError> {
        let table_id = self.metadata.get_table_id(&table_name)?;
        let context = self
            .tables
            .get_mut(&table_id)
            .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

        let inserted_count = handle_insert(
            &self.metadata,
            &mut context.table_heap,
            &mut context.buffer_pool_manager,
            &mut context.index_registry,
            &mut context.auto_increment_counters,
            table_id,
            rows,
        )?;
        self.save_to_disk()?;
        Ok(QueryResult::Inserted(inserted_count))
    }
    pub fn update(
        &mut self,
        table_name: &str,
        assign: HashMap<String, Expr>,
        predicate: Option<Expr>,
    ) -> Result<QueryResult, DomainError> {
        let table_id = self.metadata.get_table_id(&table_name)?;

        let context = self
            .tables
            .get_mut(&table_id)
            .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

        let mut assignments = HashMap::new();
        for (name, expr) in assign {
            let col_id = self.metadata.get_column_id(table_id, &name)?;

            // Bind juga ekspresi di assignment jika mengandung kolom
            let bound_expr = bind_expr(&expr, &|col_name| {
                let schema_cols = self.metadata.get_schema_columns(table_id)?;
                schema_cols
                    .iter()
                    .position(|col| col.name == col_name)
                    .ok_or_else(|| {
                        DomainError::eval_error(format!("Kolom '{col_name}' tidak ditemukan"))
                    })
            })?;

            assignments.insert(col_id, bound_expr);
        }

        // 1. Bind predicate jika ada agar Expr::Column berubah menjadi Expr::ColumnIndex
        let bound_predicate = match predicate {
            Some(expr) => {
                let bound = bind_expr(&expr, &|col_name| {
                    // Ambil posisi index kolom berdasarkan nama dari skema/katalog tabel
                    let schema_cols = self.metadata.get_schema_columns(table_id)?;
                    schema_cols
                        .iter()
                        .position(|col| col.name == col_name)
                        .ok_or_else(|| {
                            DomainError::eval_error(format!("Kolom '{col_name}' tidak ditemukan"))
                        })
                })?;
                Some(bound)
            }
            None => None,
        };

        // 2. Kirim bound_predicate.as_ref() ke handle_update
        let updated_count = handle_update(
            &self.metadata,
            &mut context.table_heap,
            &mut context.buffer_pool_manager,
            &mut context.index_registry,
            table_id,
            &assignments,
            bound_predicate.as_ref(),
        )?;

        Ok(QueryResult::Updated(updated_count))
    }

    pub fn delete(
        &mut self,
        table_name: &str,
        predicate: Option<Expr>,
    ) -> Result<QueryResult, DomainError> {
        let table_id = self.metadata.get_table_id(&table_name)?;

        let context = self
            .tables
            .get_mut(&table_id)
            .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

        let deleted_count = handle_delete(
            &self.metadata,
            &mut context.table_heap,
            &mut context.buffer_pool_manager,
            &mut context.index_registry,
            table_id,
            predicate.as_ref(),
        )?;
        Ok(QueryResult::Deleted(deleted_count))
    }

    // --- DQL
    pub fn select(
        &mut self,
        table_name: &str,
        statement: Statement,
    ) -> Result<QueryResult, DomainError> {
        let table_id = self.metadata.get_table_id(table_name)?;

        // Ambil konteks tabel dan jalankan eksekusi query[span_5](start_span)[span_5](end_span)
        let table_context = self
            .tables
            .get_mut(&table_id)
            .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

        execute_select(
            &self.metadata,
            &table_context.table_heap,
            &mut table_context.buffer_pool_manager,
            table_id,
            statement,
        )
    }
}
