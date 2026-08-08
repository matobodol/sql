use std::collections::HashMap;
use std::sync::Arc;

use crate::DomainError;
use crate::catalog::catalog_store::CatalogStore;
use crate::id::TableId;
use crate::storage::table_store::TableStorage;

#[derive(Debug, Default)]
pub struct Database {
    catalog: CatalogStore,
    tables: HashMap<TableId, TableStorage>,
}

impl Database {
    /// Mengembalikan mutable reference ke CatalogStore dan Tables sekaligus
    pub fn catalog_and_tables_mut(
        &mut self,
    ) -> (&mut CatalogStore, &mut HashMap<TableId, TableStorage>) {
        (&mut self.catalog, &mut self.tables)
    }
}

impl Database {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn catalog(&self) -> &CatalogStore {
        &self.catalog
    }

    #[inline]
    pub fn catalog_mut(&mut self) -> &mut CatalogStore {
        &mut self.catalog
    }

    #[inline]
    pub fn tables(&self) -> &HashMap<TableId, TableStorage> {
        &self.tables
    }

    #[inline]
    pub fn tables_mut(&mut self) -> &mut HashMap<TableId, TableStorage> {
        &mut self.tables
    }

    pub fn get_table_storage(&self, table_name: &str) -> Result<&TableStorage, DomainError> {
        let table_id = self
            .catalog
            .get_table_id(table_name)
            .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

        self.tables
            .get(&table_id)
            .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))
    }

    pub fn get_table_storage_mut(
        &mut self,
        table_name: &str,
    ) -> Result<&mut TableStorage, DomainError> {
        let table_id = self
            .catalog
            .get_table_id(table_name)
            .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

        self.tables
            .get_mut(&table_id)
            .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))
    }
}
