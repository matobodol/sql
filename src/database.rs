use std::sync::Arc;
use std::{collections::HashMap, path::Path};

use crate::TableContext;
use crate::disk::{BufferPoolManager, DiskManager, TableHeap};
use crate::index::IndexRegistry;
use crate::storage::DiskStorage;
use crate::{
    BASE_PATH, CommandAction, DomainError, QueryResult, TableId,
    catalog::CatalogStore,
    command::{execute_alter_table, execute_dml_action, execute_table_action},
    logic::{execute_describe_table, execute_select, execute_show_tables},
};

#[derive(Debug)]
pub struct Database {
    catalog: CatalogStore,
    // Memetakan TableId ke konteks tabel (menyimpan data di file .db terpisah)
    tables: HashMap<TableId, TableContext>,
    db_path: String,
}

impl Database {
    /// Membuat instance Database baru dengan path direktori fisik tertentu
    pub fn new(username: &str, dbname: &str) -> Self {
        let db_path = format!(
            "{BASE_PATH}/{}/{}",
            username.to_lowercase(),
            dbname.to_lowercase()
        );

        // Buat folder fisik untuk database secara otomatis saat diinisialisasi
        let _ = std::fs::create_dir_all(&db_path);

        Self {
            catalog: CatalogStore::new(),
            tables: HashMap::new(),
            db_path,
        }
    }

    /// Menyimpan metadata ke metadata.bin dan meng-flush semua halaman tabel ke file .db masing-masing
    pub fn save_to_disk(&mut self) -> Result<(), DomainError> {
        let path = Path::new(&self.db_path);
        std::fs::create_dir_all(path).map_err(|e| DomainError::storage(e.to_string()))?;

        // 1. Simpan metadata katalog (`metadata.bin`)
        let metadata_path = path.join("metadata.bin");
        DiskStorage::save_to_file(&metadata_path, &self.catalog)?;

        // 2. Flush semua halaman buffer pool tabel ke file masing-masing (`{table_name}.db`)
        for (table_id, context) in &mut self.tables {
            let _table_name = self.catalog.get_table_name(*table_id)?;
            context.buffer_pool_manager.flush_all_pages()?;
        }

        Ok(())
    }

    /// Memuat database dari struktur folder disk yang diminta
    pub fn load_from_disk(username: &str, dbname: &str) -> Result<Self, DomainError> {
        let db_path = format!(
            "{BASE_PATH}/{}/{}",
            username.to_lowercase(),
            dbname.to_lowercase()
        );
        let path = Path::new(&db_path);

        // 1. Load metadata katalog (`metadata.bin`)[span_2](start_span)[span_2](end_span)
        let metadata_path = path.join("metadata.bin");
        let catalog: CatalogStore = DiskStorage::load_from_file(&metadata_path)?;

        let mut tables = HashMap::new();

        // 2. Load setiap tabel fisik (`users.db`, `karyawan.db`, dll.)
        for table_name in catalog.list_tables() {
            let table_id = catalog.get_table_id(&table_name)?;
            let table_file_path = path.join(format!("{}.db", table_name));

            if table_file_path.exists() {
                // Inisialisasi DiskManager dan BufferPoolManager untuk file tabel tersebut[span_1](start_span)[span_1](end_span)
                let disk_manager = DiskManager::new(&table_file_path)?;
                let buffer_pool_manager = BufferPoolManager::new(disk_manager, 10);

                // Asumsikan halaman pertama adalah awal dari TableHeap[span_2](start_span)[span_2](end_span)
                let table_heap = TableHeap::from_first_page(0);

                // Inisialisasi index_registry
                let index_registry = IndexRegistry::new();

                // --- TAMBAHAN: Inisialisasi auto_increment_counters dari Schema ---
                let schema = catalog.get_schema(table_id)?;
                let mut auto_increment_counters = HashMap::new();

                for col in schema.columns() {
                    if let Some(crate::schema::Increment::Enabled { start, .. }) =
                        col.auto_increment_config()
                    {
                        auto_increment_counters.insert(col.id, *start);
                    }
                }
                // ----------------------------------------------------------------

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
            catalog,
            tables,
            db_path,
        })
    }
}

impl Database {
    /// Pintu masuk utama (facade method) untuk mengeksekusi berbagai macam `CommandAction`.
    pub fn execute(&mut self, action: CommandAction) -> Result<QueryResult, DomainError> {
        match action {
            CommandAction::ShowTables => {
                let table_names = self.catalog.list_tables();
                execute_show_tables(&table_names)
            }

            CommandAction::DescribeTable { table_name } => {
                let table_id = self.catalog.get_table_id(&table_name)?;
                let schema = self.catalog.get_schema(table_id)?;
                execute_describe_table(schema.columns())
            }

            CommandAction::TableAction { actions } => {
                // Pastikan fungsi execute_table_action menerima parameter base_path jika diperlukan
                execute_table_action(&mut self.catalog, &mut self.tables, &self.db_path, actions)?;

                self.save_to_disk()?;
                Ok(QueryResult::OK)
            }

            CommandAction::AlterTable {
                table_name,
                actions,
            } => {
                execute_alter_table(&mut self.catalog, &mut self.tables, &table_name, actions)?;

                self.save_to_disk()?;
                Ok(QueryResult::OK)
            }

            CommandAction::DmlAction { table_name, action } => {
                let table_id = self.catalog.get_table_id(&table_name)?;
                let context = self
                    .tables
                    .get_mut(&table_id)
                    .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name.clone())))?;

                let result = execute_dml_action(
                    &self.catalog,
                    &mut context.table_heap,
                    &mut context.buffer_pool_manager,
                    &mut context.index_registry,
                    &mut context.auto_increment_counters, // <-- Tambahkan parameter ini ke execute_dml_action
                    table_id,
                    action,
                )?;

                self.save_to_disk()?;
                Ok(result)
            }

            CommandAction::Select {
                table_name,
                statements,
            } => {
                let table_id = self.catalog.get_table_id(&table_name)?;

                // Ubah .get() menjadi .get_mut() agar buffer_pool_manager bisa dipinjam secara mutabel
                let table_context = self
                    .tables
                    .get_mut(&table_id)
                    .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name.clone())))?;

                execute_select(
                    &self.catalog,
                    &table_context.table_heap,
                    &mut table_context.buffer_pool_manager, // Sisipkan BufferPoolManager di argumen ke-3
                    table_id,
                    statements,
                )
            }
        }
    }

    // ==========================================
    // GETTERS & HELPERS
    // ==========================================

    /// Mengambil referensi ke katalog metadata.
    pub fn catalog(&self) -> &CatalogStore {
        &self.catalog
    }

    /// Mengambil referensi mutabel ke katalog metadata.
    pub fn catalog_mut(&mut self) -> &mut CatalogStore {
        &mut self.catalog
    }

    /// Mengambil referensi ke peta penyimpanan tabel fisik (`TableContext`).
    pub fn tables(&self) -> &HashMap<TableId, TableContext> {
        &self.tables
    }

    /// Mendapatkan akses mutabel ke konteks tabel spesifik berdasarkan ID-nya.
    pub fn table_context_mut(&mut self, table_id: TableId) -> Option<&mut TableContext> {
        self.tables.get_mut(&table_id)
    }
}
