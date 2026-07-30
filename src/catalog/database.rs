use crate::catalog::registry::SymbolRegistry;
use crate::catalog::table::Table;
use crate::domain::id::TableId;
use crate::domain::{ColumnDef, DomainError, Schema};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Database {
    registry: SymbolRegistry,
    tables: HashMap<TableId, Table>,
}

impl Database {
    pub fn new() -> Self {
        Self {
            registry: SymbolRegistry::new(),
            tables: HashMap::new(),
        }
    }

    /// Accessor untuk SymbolRegistry (Source of Truth)
    pub fn registry(&self) -> &SymbolRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut SymbolRegistry {
        &mut self.registry
    }

    /// Membuat tabel baru dengan mendaftarkan Table & Columns ke Registry
    pub fn create_table(
        &mut self,
        table_name: &str,
        raw_columns: Vec<(
            String,
            crate::domain::SqlType,
            Vec<crate::domain::ColumnConstraint>,
        )>,
    ) -> Result<TableId, DomainError> {
        // 1. Register Nama Tabel ke Registry -> Mengembalikan TableId
        let table_id = self.registry.register_table(table_name)?;

        // 2. Register Setiap Nama Kolom ke Registry -> Mengembalikan ColumnId unik
        let mut column_defs = Vec::with_capacity(raw_columns.len());
        for (col_name, sql_type, constraints) in raw_columns {
            let col_id = self.registry.register_column(&col_name);
            column_defs.push(ColumnDef::with_constraints(
                col_id,
                col_name,
                sql_type,
                constraints,
            ));
        }

        // 3. Buat Schema atomik
        let schema = Schema::new(column_defs)?;
        let table = Table::new(table_id, table_name, schema);

        self.tables.insert(table_id, table);
        Ok(table_id)
    }

    /// Mencari tabel berdasarkan nama string via Registry Lookup
    pub fn get_table(&self, table_name: &str) -> Result<&Table, DomainError> {
        let table_id = self
            .registry
            .get_table_id(table_name)
            .ok_or_else(|| DomainError::TableNotFound(table_name.to_string()))?;

        self.tables
            .get(&table_id)
            .ok_or_else(|| DomainError::TableNotFound(table_name.to_string()))
    }

    pub fn get_table_mut(&mut self, table_name: &str) -> Result<&mut Table, DomainError> {
        let table_id = self
            .registry
            .get_table_id(table_name)
            .ok_or_else(|| DomainError::TableNotFound(table_name.to_string()))?;

        self.tables
            .get_mut(&table_id)
            .ok_or_else(|| DomainError::TableNotFound(table_name.to_string()))
    }

    /// RENAME COLUMN: Mengubah nama di Registry tanpa menyentuh data fisik di Table!
    pub fn rename_column(&mut self, old_name: &str, new_name: &str) -> Result<(), DomainError> {
        self.registry.rename_column(old_name, new_name)
    }
}
