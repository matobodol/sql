use crate::id::{ColumnId, TableId};
use crate::index::IndexRegistry;
use crate::{AutoIncrement, Column, ColumnConstraint, DomainError, RowStore, SqlValue};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct TableStorage {
    id: TableId,
    name: String,
    /// Penyimpanan baris terisolasi
    row_store: RowStore,
    /// Registry indeks B-Tree
    index_registry: IndexRegistry,
    /// Counter auto-increment berbasis ColumnId
    auto_increment_counters: HashMap<ColumnId, i64>,
}

impl TableStorage {
    /// Inisialisasi fisik tabel menggunakan Zero-Copy Arc metadata dari CatalogStore
    pub fn new_with_arc(id: TableId, name: impl Into<String>, schema_cols: Arc<[Column]>) -> Self {
        let mut auto_increment_counters = HashMap::new();

        // Inisialisasi counter auto-increment langsung dari Arc slice
        for col in schema_cols.iter() {
            if let Some(AutoIncrement::Enabled { start, .. }) = col.auto_increment_config() {
                auto_increment_counters.insert(col.id, *start);
            }
        }

        let mut table = Self {
            id,
            name: name.into(),
            row_store: RowStore::new(),
            index_registry: IndexRegistry::new(),
            auto_increment_counters,
        };

        table.build_indexes_from_schema(&schema_cols);
        table
    }

    /// Membuat indeks unik awal dari slice metadata katalog
    fn build_indexes_from_schema(&mut self, schema_cols: &[Column]) {
        for col in schema_cols {
            let is_unique = col.is_primary_key()
                || col
                    .constraints
                    .iter()
                    .any(|c| matches!(c, ColumnConstraint::Unique));

            if is_unique {
                let _ = self.index_registry.create_btree_index(col.id, true);
            }
        }
    }

    pub fn id(&self) -> TableId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    #[inline]
    pub fn row_store(&self) -> &RowStore {
        &self.row_store
    }

    #[inline]
    pub fn row_store_mut(&mut self) -> &mut RowStore {
        &mut self.row_store
    }

    #[inline]
    pub fn index_registry(&self) -> &IndexRegistry {
        &self.index_registry
    }

    #[inline]
    pub fn index_registry_mut(&mut self) -> &mut IndexRegistry {
        &mut self.index_registry
    }

    #[inline]
    pub fn auto_increment_counters(&self) -> &HashMap<ColumnId, i64> {
        &self.auto_increment_counters
    }

    #[inline]
    pub fn auto_increment_counters_mut(&mut self) -> &mut HashMap<ColumnId, i64> {
        &mut self.auto_increment_counters
    }

    /// Rekonstruksi ulang indeks B-Tree secara Zero-Copy tanpa mengkloning SqlValue
    pub fn rebuild_indexes(&mut self, schema_cols: &[Column]) -> Result<(), DomainError> {
        self.index_registry.clear_entries();

        // Ambil pemetaan indeks kolom secara konstan O(1)
        let col_ids: Vec<ColumnId> = schema_cols.iter().map(|c| c.id).collect();

        for row in self.row_store.rows() {
            let row_id = row.id();

            // Mengirim pasangan borrow (&ColumnId, &SqlValue) tanpa .clone()
            let entries: Vec<(ColumnId, &SqlValue)> = col_ids
                .iter()
                .zip(row.values())
                .map(|(&col_id, val)| (col_id, val))
                .collect();

            self.index_registry.insert_entry_ref(row_id, &entries)?;
        }

        Ok(())
    }
}
