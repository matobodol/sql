use crate::catalog::db_function::alter_table::AlterTableAction;
use crate::catalog::registry::SymbolRegistry;
use crate::catalog::table::Table;
use crate::domain::id::TableId;
use crate::domain::{ColumnConstraint, ColumnDef, DomainError, Schema, SqlType};
use crate::{QueryResult, SelectStmt, execute_alter, execute_select};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
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

    // --- ACCESSORS (Source of Truth) ---

    pub fn registry(&self) -> &SymbolRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut SymbolRegistry {
        &mut self.registry
    }

    pub fn tables(&self) -> &HashMap<TableId, Table> {
        &self.tables
    }

    pub fn tables_mut(&mut self) -> &mut HashMap<TableId, Table> {
        &mut self.tables
    }

    // --- TABLE MANAGEMENT ---

    /// Membuat tabel baru dengan mendaftarkan Table & Columns ke Registry
    pub fn create_table(
        &mut self,
        table_name: &str,
        raw_columns: Vec<(String, SqlType, Vec<ColumnConstraint>)>,
    ) -> Result<TableId, DomainError> {
        // 1. Register Nama Tabel ke Registry -> Mengembalikan TableId
        let table_id = self.registry.register_table(table_name)?;

        // 2. Register Setiap Nama Kolom terikat dengan TableId
        let mut column_defs = Vec::with_capacity(raw_columns.len());
        for (col_name, sql_type, constraints) in raw_columns {
            let col_id = self.registry.register_column(table_id, &col_name);
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

    // --- DDL ENGINE INTEGRATION ---

    /// Pintu masuk utama eksekusi ALTER TABLE (Multi-action & Staging Atomic)
    pub fn execute_alter(
        &mut self,
        table_name: &str,
        actions: Vec<AlterTableAction>,
    ) -> Result<(), DomainError> {
        execute_alter(self, table_name, actions)
    }

    /// Pintu masuk utama untuk seluruh aksi DQL (SELECT).
    /// Menggunakan `&self` (read-only) untuk mendukung concurrent reads dan zero-copy scanning.
    pub fn execute_select(
        &self,
        table_name: &str,
        stmt: SelectStmt,
    ) -> Result<QueryResult, DomainError> {
        execute_select(self, table_name, stmt)
    }
}
