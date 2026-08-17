use std::collections::HashMap;

use crate::{
    AggregateFunc, ColumnConstraint, ColumnPosition, DataType, DatabaseManager, DomainError, Expr,
    OrderByExpr, ValueType,
    api_command::{CMD, execute},
    catalog::QueryResult,
    execution::SelectStatement,
    logic::{Statement, table_action::virtual_column},
};

type DBMError = Result<QueryResult, DomainError>;

/// `DBM` (Database Manager) adalah titik masuk utama untuk berinteraksi dengan sistem database.
///
/// Struktur ini menyediakan metode untuk menjalankan perintah SQL melalui antarmuka
/// yang terisolasi dari implementasi internal `DatabaseManager`.
#[derive(Debug)]
pub struct DBM {
    dbm: DatabaseManager,
}

impl DBM {
    /// Membuat instance baru dari `DBM` dengan inisialisasi sistem database internal.
    pub fn new() -> Self {
        Self {
            dbm: DatabaseManager::new(),
        }
    }

    /// Mengeksekusi sekumpulan perintah (`CMD`) terhadap database aktif.
    ///
    /// # Arguments
    /// * `cmd` - Vektor berisi perintah yang akan dijalankan oleh mesin database.
    ///
    /// # Returns
    /// * `QueryResult` jika operasi berhasil.
    /// * `DomainError` jika terjadi kesalahan dalam logika database.
    ///
    /// # Example
    /// ```rust
    /// use sql::{DBM, CMD};
    ///
    /// let mut db = DBM::new();
    /// db.execute(vec![CMD::CreateDatabase{db_name: "mydb".to_string()}]);
    /// ```
    ///
    /// Gunakan konstruktor :
    /// build_statement() dan build_agregate_func
    /// untuk membangun execute elect
    ///
    pub fn execute(&mut self, cmd: Vec<CMD>) -> DBMError {
        execute(self, cmd)
    }
}

impl DBM {
    // -- DATABASE
    pub(crate) fn create_database(&mut self, db_name: &str) -> DBMError {
        self.dbm.create_database(db_name)?;
        Ok(QueryResult::OK)
    }
    pub(crate) fn rename_database(&mut self, old_name: &str, new_name: &str) -> DBMError {
        self.dbm.rename_database(old_name, new_name)?;
        Ok(QueryResult::OK)
    }
    pub(crate) fn drop_database(&mut self, db_name: &str) -> DBMError {
        self.dbm.drop_database(db_name)?;
        Ok(QueryResult::OK)
    }
    pub(crate) fn show_databases(&self) -> DBMError {
        let (schema, rows) = virtual_column(self.dbm.list_databases())?;
        Ok(QueryResult::Dql { schema, rows })
    }
    pub(crate) fn use_database(&mut self, db_name: &str) -> DBMError {
        self.dbm.use_database(db_name)?;
        Ok(QueryResult::OK)
    }

    // -- USER
    pub(crate) fn create_user(&mut self, username: &str, password_hash: &str) -> DBMError {
        self.dbm.create_user(username, password_hash)?;
        Ok(QueryResult::OK)
    }
    pub(crate) fn user_login(&mut self, username: &str, password_hash: &str) -> DBMError {
        self.dbm.login(username, password_hash)?;
        Ok(QueryResult::OK)
    }
    pub(crate) fn change_password(
        &mut self,
        old_password: Option<String>,
        new_password: &str,
    ) -> DBMError {
        self.dbm.change_password(old_password, new_password)?;
        Ok(QueryResult::OK)
    }
    pub(crate) fn rename_user(&mut self, old_username: &str, new_username: &str) -> DBMError {
        self.dbm.rename_user(old_username, new_username)?;
        Ok(QueryResult::OK)
    }
    pub(crate) fn drop_user(&mut self, username: &str) -> DBMError {
        self.dbm.drop_user(username)?;
        Ok(QueryResult::OK)
    }
    pub(crate) fn show_users(&self) -> DBMError {
        let (schema, rows) = virtual_column(self.dbm.show_users())?;
        Ok(QueryResult::Dql { schema, rows })
    }

    // -- TABLE
    pub(crate) fn create_table(
        &mut self,
        table_name: &str,
        raw_columns: Vec<(String, DataType, Vec<ColumnConstraint>)>,
    ) -> DBMError {
        let db_mut = self.dbm.active_database_mut()?;
        db_mut.create_table(table_name, raw_columns)?;
        Ok(QueryResult::OK)
    }
    pub(crate) fn rename_table(&mut self, old_name: &str, new_name: &str) -> DBMError {
        let db_mut = self.dbm.active_database_mut()?;
        db_mut.rename_table(old_name, new_name)?;
        Ok(QueryResult::OK)
    }
    pub(crate) fn drop_table(&mut self, table_name: &str) -> DBMError {
        let db_mut = self.dbm.active_database_mut()?;
        db_mut.drop_table(table_name)?;
        Ok(QueryResult::OK)
    }
    pub(crate) fn show_tables(&self) -> DBMError {
        let (schema, rows) = self.dbm.active_database_ref()?.show_tables()?;
        Ok(QueryResult::Dql { schema, rows })
    }
    pub(crate) fn describe_table(&self, table_name: &str) -> DBMError {
        let db_ref = self.dbm.active_database_ref()?;
        db_ref.describe_table(table_name)
    }

    // -- DDL
    pub(crate) fn add_columns(
        &mut self,
        table_name: &str,
        raw_columns: Vec<(String, DataType, Vec<ColumnConstraint>, ColumnPosition)>,
    ) -> DBMError {
        let db_mut = self.dbm.active_database_mut()?;
        db_mut.add_columns(table_name, raw_columns)?;
        Ok(QueryResult::OK)
    }
    pub(crate) fn drop_column(&mut self, table_name: &str, col_name: &str) -> DBMError {
        let db_mut = self.dbm.active_database_mut()?;
        db_mut.drop_column(table_name, col_name)?;
        Ok(QueryResult::OK)
    }
    pub(crate) fn rename_column(
        &mut self,
        table_name: &str,
        old_name: &str,
        new_name: &str,
    ) -> DBMError {
        let db_mut = self.dbm.active_database_mut()?;
        db_mut.rename_column(table_name, old_name, new_name)?;
        Ok(QueryResult::OK)
    }
    pub(crate) fn modify_column_type(
        &mut self,
        table_name: &str,
        col_name: &str,
        new_type: DataType,
    ) -> DBMError {
        let db_mut = self.dbm.active_database_mut()?;
        db_mut.modify_column_type(table_name, col_name, new_type)?;
        Ok(QueryResult::OK)
    }
    pub(crate) fn add_column_constraint(
        &mut self,
        table_name: &str,
        col_name: &str,
        constraint: ColumnConstraint,
    ) -> DBMError {
        let db_mut = self.dbm.active_database_mut()?;
        db_mut.add_column_constraint(table_name, col_name, constraint)?;
        Ok(QueryResult::OK)
    }
    pub(crate) fn drop_column_constraint(
        &mut self,
        table_name: &str,
        col_name: &str,
        constraint: ColumnConstraint,
    ) -> DBMError {
        let db_mut = self.dbm.active_database_mut()?;
        db_mut.drop_column_constraint(table_name, col_name, constraint)?;
        Ok(QueryResult::OK)
    }
    pub(crate) fn set_default_value(
        &mut self,
        table_name: &str,
        col_name: &str,
        default_val: Option<ValueType>,
    ) -> DBMError {
        let db_mut = self.dbm.active_database_mut()?;
        db_mut.set_default(table_name, col_name, default_val)?;
        Ok(QueryResult::OK)
    }

    // -- DML
    pub(crate) fn insert_rows(&mut self, table_name: &str, rows: Vec<Vec<ValueType>>) -> DBMError {
        let db_mut = self.dbm.active_database_mut()?;
        db_mut.insert(table_name, rows)
    }
    pub(crate) fn update_rows(
        &mut self,
        table_name: &str,
        assignments: HashMap<String, Expr>,
        predicate: Option<Expr>,
    ) -> DBMError {
        let db_mut = self.dbm.active_database_mut()?;
        db_mut.update(table_name, assignments, predicate)
    }
    pub(crate) fn delete_rows(&mut self, table_name: &str, predicate: Option<Expr>) -> DBMError {
        let db_mut = self.dbm.active_database_mut()?;
        db_mut.delete(table_name, predicate)
    }

    // -- DQL
    pub(crate) fn select(&mut self, table_name: &str, stmt: Statement) -> DBMError {
        let db_mut = self.dbm.active_database_mut()?;
        db_mut.select(table_name, stmt)
    }
}

impl DBM {
    pub fn build_statement(
        &self,
        table_name: &str,
        projection: Vec<Expr>,
        // Kondisi penyaringan baris data (klausa WHERE).
        selection: Option<Expr>,
        // Daftar ID kolom yang digunakan untuk pengelompokan data (klausa GROUP BY).
        group_by: Vec<String>,
        // Daftar fungsi agregasi yang diterapkan (misalnya SUM, COUNT, AVG, MIN, MAX).
        aggregates: Vec<AggregateFunc>,
        // Pengaturan pengurutan baris hasil query (klausa ORDER BY).
        order_by: Vec<OrderByExpr>,
        // Batas jumlah maksimum baris yang dikembalikan (klausa LIMIT).
        limit: Option<usize>,

        // Jumlah baris awal yang dilewati sebelum mulai mengembalikan hasil (klausa OFFSET).
        offset: usize,
    ) -> Result<SelectStatement, DomainError> {
        let db_ref = self.dbm.active_database_ref()?;
        let meta = db_ref.meta();
        let table_id = meta.get_table_id(table_name)?;

        // Closure standar untuk menerjemahkan nama kolom string menjadi ColumnId via katalog
        let get_col_id = |name: &str| meta.get_column_id(table_id, name);

        // 1. Bind group_by dari Vec<String> ke Vec<ColumnId>
        let mut fixed_group_by = Vec::with_capacity(group_by.len());
        for name in &group_by {
            fixed_group_by.push(get_col_id(name)?);
        }

        let stmt = SelectStatement {
            projection,
            selection,
            group_by: fixed_group_by,
            aggregates,
            order_by,
            limit,
            offset,
        };

        Ok(stmt)
    }

    pub fn build_agregate_func(
        &self,
        table_name: &str,
        count: Option<String>,
        sum: Option<String>,
        avg: Option<String>,
        min: Option<String>,
        max: Option<String>,
    ) -> Result<Vec<AggregateFunc>, DomainError> {
        let db_ref = self.dbm.active_database_ref()?;
        let meta = db_ref.meta();
        let table_id = meta.get_table_id(table_name)?;

        // Closure standar untuk menerjemahkan nama kolom string menjadi ColumnId via katalog
        let get_col_id = |name: &str| meta.get_column_id(table_id, name);

        let mut aggregates = Vec::new();

        if let Some(name) = count {
            let id = get_col_id(&name)?;
            aggregates.push(AggregateFunc::Count(Some(id)))
        } else {
            aggregates.push(AggregateFunc::Count(None));
        }

        if let Some(name) = sum {
            let id = get_col_id(&name)?;
            aggregates.push(AggregateFunc::Sum(id));
        }

        if let Some(name) = avg {
            let id = get_col_id(&name)?;
            aggregates.push(AggregateFunc::Avg(id));
        }

        if let Some(name) = min {
            let id = get_col_id(&name)?;
            aggregates.push(AggregateFunc::Min(id));
        }

        if let Some(name) = max {
            let id = get_col_id(&name)?;
            aggregates.push(AggregateFunc::Max(id));
        }

        Ok(aggregates)
    }
}
