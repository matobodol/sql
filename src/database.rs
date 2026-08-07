use std::collections::HashMap;
use std::sync::Arc;

use crate::catalog::CatalogStore;
use crate::command::{ColumnPosition, CommandAction, QueryResult, execute_command};
use crate::id::{ColumnId, TableId};
use crate::query_logic::{ddl_action, dml_action, dql_action};
use crate::table_store::TableStorage;
use crate::{ColumnConstraint, DomainError, Expr, SelectStmt, SqlType, SqlValue};

/// Facade Engine Basis Data utama yang mengordinasikan Katalog Metadata,
/// Penyimpanan Fisik Tabel, dan Eksekusi Perintah.
#[derive(Debug, Default)]
pub struct Database {
    catalog: CatalogStore,
    tables: HashMap<TableId, TableStorage>,
}

impl Database {
    /// Membuat instance Database baru dengan katalog dan tabel kosong.
    #[inline]
    pub fn new() -> Self {
        Self {
            catalog: CatalogStore::new(),
            tables: HashMap::new(),
        }
    }

    // ------------------------------------------------------------------
    // ACCESSOR & MUTATOR UTAMA
    // ------------------------------------------------------------------

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

    /// Mendapatkan rujukan mutable ke catalog dan tables secara bersamaan (*disjoint borrow*).
    #[inline]
    pub fn catalog_and_tables_mut(
        &mut self,
    ) -> (&mut CatalogStore, &mut HashMap<TableId, TableStorage>) {
        (&mut self.catalog, &mut self.tables)
    }

    /// Ambil referensi immutable `TableStorage` berdasarkan nama tabel
    pub fn get_table_storage(&self, name: &str) -> Result<&TableStorage, DomainError> {
        let table_id = self
            .catalog
            .get_table_id(name)
            .ok_or_else(|| DomainError::TableNotFound(Arc::from(name)))?;

        self.tables
            .get(&table_id)
            .ok_or_else(|| DomainError::TableNotFound(Arc::from(name)))
    }

    /// Ambil referensi mutable `TableStorage` berdasarkan nama tabel
    pub fn get_table_storage_mut(&mut self, name: &str) -> Result<&mut TableStorage, DomainError> {
        let table_id = self
            .catalog
            .get_table_id(name)
            .ok_or_else(|| DomainError::TableNotFound(Arc::from(name)))?;

        self.tables
            .get_mut(&table_id)
            .ok_or_else(|| DomainError::TableNotFound(Arc::from(name)))
    }

    // ------------------------------------------------------------------
    // PUBLIC FACADE API (UNTUK INTEGRASI CRATE LUAR)
    // ------------------------------------------------------------------

    /// Eksekusi perintah berstruktur (DDL, DML, DQL) secara terpusat.
    #[inline]
    pub fn execute(
        &mut self,
        table_name: &str,
        action: CommandAction,
    ) -> Result<QueryResult, DomainError> {
        execute_command(self, table_name, action)
    }

    /// Menjalankan query DQL (`SELECT`).
    #[inline]
    pub fn select(&self, table_name: &str, stmt: SelectStmt) -> Result<QueryResult, DomainError> {
        dql_action::execute_select(self, table_name, stmt)
    }

    /// Menampilkan daftar seluruh tabel yang terdaftar di katalog.
    #[inline]
    pub fn show_tables(&self) -> Result<QueryResult, DomainError> {
        dql_action::show_tables(self)
    }

    /// Menyisipkan data ke dalam tabel (DML `INSERT`).
    #[inline]
    pub fn insert(
        &mut self,
        table_name: &str,
        rows: Vec<Vec<SqlValue>>,
    ) -> Result<usize, DomainError> {
        dml_action::handle_insert(self, table_name, rows)
    }

    /// Memperbarui data dalam tabel (DML `UPDATE`).
    #[inline]
    pub fn update(
        &mut self,
        table_name: &str,
        assignments: &HashMap<ColumnId, Expr>,
        predicate: Option<&Expr>,
    ) -> Result<usize, DomainError> {
        dml_action::handle_update(self, table_name, assignments, predicate)
    }

    /// Menghapus data dari tabel (DML `DELETE`).
    #[inline]
    pub fn delete(
        &mut self,
        table_name: &str,
        predicate: Option<&Expr>,
    ) -> Result<usize, DomainError> {
        dml_action::handle_delete(self, table_name, predicate)
    }

    /// Membuat tabel baru (DDL `CREATE TABLE`).
    #[inline]
    pub fn create_table(
        &mut self,
        table_name: &str,
        columns: Vec<(String, SqlType, Vec<ColumnConstraint>)>,
    ) -> Result<TableId, DomainError> {
        let (catalog, tables) = self.catalog_and_tables_mut();
        ddl_action::create_table(catalog, tables, table_name, columns)
    }

    /// Menghapus tabel (DDL `DROP TABLE`).
    #[inline]
    pub fn drop_table(&mut self, table_name: &str) -> Result<(), DomainError> {
        let (catalog, tables) = self.catalog_and_tables_mut();
        ddl_action::drop_table(catalog, tables, table_name)
    }

    /// Menambah kolom ke tabel (DDL `ALTER TABLE ADD COLUMN`).
    #[inline]
    pub fn add_columns(
        &mut self,
        table_name: &str,
        columns: Vec<(String, SqlType, Vec<ColumnConstraint>, ColumnPosition)>,
    ) -> Result<(), DomainError> {
        let (catalog, tables) = self.catalog_and_tables_mut();
        ddl_action::execute_add_columns(catalog, tables, table_name, columns)
    }

    /// Fungsi delegasi lama untuk kompatibilitas caller / legacy engine.
    #[inline]
    pub fn execute_ddl(
        db: &mut Database,
        _catalog: &mut CatalogStore,
        table_name: &str,
        action: CommandAction,
    ) -> Result<QueryResult, DomainError> {
        execute_command(db, table_name, action)
    }
}
