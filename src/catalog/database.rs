use crate::catalog::db_function::ddl_action::AlterTableAction;
use crate::catalog::registry::SymbolRegistry;
use crate::catalog::table::Table;
use crate::domain::id::TableId;
use crate::domain::{DomainError, Schema};
use crate::{
    ColumnConstraint, ColumnDef, ColumnId, DmlAction, DmlResult, QueryResult, SelectStmt, SqlType,
    SqlValue, execute_alter, execute_select,
};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct Database {
    registry: SymbolRegistry,
    tables: HashMap<TableId, Table>,
}

impl Database {
    // =========================================================================
    // CRATE-VISIBILITY ACCESSORS (Hanya bisa diakses di dalam crate `sql`)
    // =========================================================================

    /// Referensi immutable ke SymbolRegistry internal
    pub(crate) fn registry(&self) -> &SymbolRegistry {
        &self.registry
    }

    /// Referensi mutable ke SymbolRegistry internal (diperlukan saat ALTER / RENAME)
    pub(crate) fn registry_mut(&mut self) -> &mut SymbolRegistry {
        &mut self.registry
    }

    /// Referensi immutable ke HashMap seluruh tabel
    pub(crate) fn _tables(&self) -> &HashMap<TableId, Table> {
        &self.tables
    }

    /// Referensi mutable ke HashMap seluruh tabel
    pub(crate) fn tables_mut(&mut self) -> &mut HashMap<TableId, Table> {
        &mut self.tables
    }
}

impl Database {
    // =========================================================================
    // PUBLIC ACCESSORS (Facade berparameter Nama String untuk Consumer/Engine)
    // =========================================================================

    // --- IDENTITY / LOOKUP API ---

    /// Mengambil TableId berdasarkan nama tabel
    pub fn get_table_id(&self, table_name: &str) -> Option<TableId> {
        self.registry.get_table_id(table_name)
    }

    /// Mengambil ColumnId berdasarkan nama tabel dan nama kolom
    pub fn get_column_id(&self, table_name: &str, col_name: &str) -> Option<ColumnId> {
        let table_id = self.get_table_id(table_name)?;
        self.registry.get_column_id(table_id, col_name)
    }

    /// Reverse Lookup: Mengambil nama tabel berdasarkan TableId
    pub fn get_table_name(&self, table_id: TableId) -> Option<&str> {
        self.registry.get_table_name(table_id)
    }
    /// Ambil referensi immutable ke `Table` berdasarkan nama string.
    /// Pengguna API luar tidak perlu tahu `TableId`.
    pub fn get_table(&self, table_name: &str) -> Result<&Table, DomainError> {
        let table_id = self
            .registry
            .get_table_id(table_name)
            .ok_or_else(|| DomainError::TableNotFound(table_name.to_string()))?;

        self.tables
            .get(&table_id)
            .ok_or_else(|| DomainError::TableNotFound(table_name.to_string()))
    }

    /// Ambil referensi mutable ke `Table` berdasarkan nama string.
    pub fn get_table_mut(&mut self, table_name: &str) -> Result<&mut Table, DomainError> {
        let table_id = self
            .registry
            .get_table_id(table_name)
            .ok_or_else(|| DomainError::TableNotFound(table_name.to_string()))?;

        self.tables
            .get_mut(&table_id)
            .ok_or_else(|| DomainError::TableNotFound(table_name.to_string()))
    }

    /// Helper instan untuk mengambil list seluruh nama tabel yang terdaftar di database
    pub fn list_tables(&self) -> Vec<String> {
        // Mengambil daftar tabel langsung dari registry atau iterasi HashMap
        // Berfungsi baik untuk perintah CLI seperti `\dt` atau `SHOW TABLES;`
        self.tables.values().map(|t| t.name().to_string()).collect()
    }
}

impl Database {
    // --- HELPER METADATA (Dipakai Engine) ---

    pub fn table_exists(&self, table_name: &str) -> bool {
        self.registry.get_table_id(table_name).is_some()
    }

    pub fn get_schema(&self, table_name: &str) -> Result<&Schema, DomainError> {
        let table = self.get_table(table_name)?;
        Ok(table.schema())
    }

    // --- DDL API ---

    /// Pintu masuk CREATE TABLE
    pub fn create_table(
        &mut self,
        table_name: &str,
        raw_columns: Vec<(String, SqlType, Vec<ColumnConstraint>)>,
    ) -> Result<TableId, DomainError> {
        // 1. Register tabel ke registry
        let table_id = self.registry.register_table(table_name)?;

        // Helper closure untuk rollback registry jika terjadi error di tengah jalan
        let build_schema = || {
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

            Schema::new(column_defs)
        };

        // 2. Buat skema. Jika GAGAL, bersihkan registry agar tidak ada ID menggantung!
        let schema = match build_schema() {
            Ok(s) => s,
            Err(err) => {
                let _ = self.registry.unregister_table(table_name);
                return Err(err);
            }
        };

        // 3. Simpan tabel ke HashMap
        let table = Table::new(table_id, table_name, schema);
        self.tables.insert(table_id, table);

        Ok(table_id)
    }

    /// Pintu masuk DROP TABLE
    pub fn drop_table(&mut self, table_name: &str) -> Result<(), DomainError> {
        let table_id = self.registry.unregister_table(table_name)?;
        self.tables.remove(&table_id);
        Ok(())
    }

    /// Pintu masuk ALTER TABLE
    pub fn execute_alter(
        &mut self,
        table_name: &str,
        actions: Vec<AlterTableAction>,
    ) -> Result<(), DomainError> {
        execute_alter(self, table_name, actions)
    }

    // --- DML API (INSERT, UPDATE, DELETE) ---

    /// Pintu masuk utama DML
    pub fn execute_dml(
        &mut self,
        table_name: &str,
        action: DmlAction,
    ) -> Result<DmlResult, DomainError> {
        let table = self.get_table_mut(table_name)?;
        table.execute_dml(action)
    }

    /// Convenience wrapper untuk INSERT single row
    pub fn insert_row(
        &mut self,
        table_name: &str,
        row_values: Vec<SqlValue>,
    ) -> Result<usize, DomainError> {
        let table = self.get_table_mut(table_name)?;
        table.insert(row_values)
    }

    /// Convenience wrapper untuk BULK INSERT
    pub fn insert_batch(
        &mut self,
        table_name: &str,
        rows: Vec<Vec<SqlValue>>,
    ) -> Result<usize, DomainError> {
        let table = self.get_table_mut(table_name)?;
        table.insert_batch(rows)
    }

    // --- DQL API (SELECT) ---

    /// Pintu masuk SELECT Query
    pub fn execute_select(
        &self,
        table_name: &str,
        stmt: SelectStmt,
    ) -> Result<QueryResult, DomainError> {
        execute_select(self, table_name, stmt)
    }
}
