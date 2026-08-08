use std::collections::HashMap;
use std::sync::Arc;

use crate::DomainError;
use crate::index::index_registry::IndexRegistry;
use crate::schema::{AutoIncrement, Schema};
use crate::storage::row_store::RowStore;
use crate::{ColumnId, TableId};

#[derive(Debug)]
pub struct TableStorage {
    table_id: TableId,
    name: String,
    schema: Arc<Schema>,
    row_store: RowStore,
    index_registry: IndexRegistry,
    auto_increment_counters: HashMap<ColumnId, i64>,
}

impl TableStorage {
    pub fn new(table_id: TableId, name: &str, schema: Arc<Schema>) -> Self {
        let mut auto_increment_counters = HashMap::new();

        for col in schema.columns() {
            if let Some(AutoIncrement::Enabled { start, .. }) = col.auto_increment_config() {
                auto_increment_counters.insert(col.id, *start);
            }
        }

        Self {
            table_id,
            name: name.to_string(),
            schema,
            row_store: RowStore::new(),
            index_registry: IndexRegistry::new(),
            auto_increment_counters,
        }
    }

    #[inline]
    pub fn table_id(&self) -> TableId {
        self.table_id
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, new_name: &str) {
        self.name = new_name.to_string();
    }

    #[inline]
    pub fn schema(&self) -> &Arc<Schema> {
        &self.schema
    }

    pub fn update_schema(&mut self, new_schema: Arc<Schema>) {
        self.schema = new_schema;
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

    pub fn rebuild_indexes(&mut self, schema: &Schema) -> Result<(), DomainError> {
        self.index_registry.clear();
        for row in self.row_store.rows() {
            let entries: Vec<(ColumnId, &crate::ValueType)> = schema
                .columns()
                .iter()
                .enumerate()
                .filter(|(_, col)| self.index_registry.has_index(col.id))
                .map(|(idx, col)| (col.id, &row.values()[idx]))
                .collect();

            self.index_registry.insert_entry_ref(row.id(), &entries)?;
        }
        Ok(())
    }
}
