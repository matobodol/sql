use std::collections::HashMap;

use crate::{DomainError, TableId, storage::table_store::TableStorage};

#[derive(Debug, Default)]
pub struct Database {
    // name: String,
    tables: HashMap<TableId, TableStorage>,
}
impl Database {
    pub fn new() -> Self {
        Self {
            // name: name.into(),
            tables: HashMap::new(),
        }
    }

    pub fn table_mut(&mut self, table: (&str, &TableId)) -> Result<&mut TableStorage, DomainError> {
        let (name, id) = table;
        self.tables
            .get_mut(id)
            .ok_or_else(|| DomainError::TableNotFound(name.into()))
    }
    pub fn tables_mut(&mut self) -> &mut HashMap<TableId, TableStorage> {
        &mut self.tables
    }
}
