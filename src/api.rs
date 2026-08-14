use std::collections::HashMap;

use crate::{
    ColumnConstraint, ColumnPosition, DataType, DatabaseManager, DomainError, Expr, SelectStmt,
    ValueType, catalog::QueryResult, logic::table_action::virtual_column,
};

type DBMError = Result<QueryResult, DomainError>;

/// API ISOLATION
/// menghindari expose mothod DatabaseManager
#[derive(Debug)]
pub struct DBM {
    dbm: DatabaseManager,
}

impl DBM {
    pub fn new() -> Self {
        Self {
            dbm: DatabaseManager::new(),
        }
    }

    // -- DATABASE
    pub fn api_database_create(&mut self, db_name: &str) -> DBMError {
        self.dbm.create_database(db_name)?;
        Ok(QueryResult::OK)
    }
    pub fn api_database_rename(&mut self, old_name: &str, new_name: &str) -> DBMError {
        self.dbm.rename_database(old_name, new_name)?;
        Ok(QueryResult::OK)
    }
    pub fn api_database_drop(&mut self, db_name: &str) -> DBMError {
        self.dbm.drop_database(db_name)?;
        Ok(QueryResult::OK)
    }
    pub fn api_databases_show(dbm: &DatabaseManager) -> DBMError {
        let (schema, rows) = virtual_column(dbm.list_databases())?;
        Ok(QueryResult::Dql { schema, rows })
    }
    pub fn api_database_use(&mut self, db_name: &str) -> DBMError {
        self.dbm.use_database(db_name)?;
        Ok(QueryResult::OK)
    }

    // -- USER
    pub fn api_user_create(
        dbm: &mut DatabaseManager,
        username: &str,
        password_hash: &str,
    ) -> DBMError {
        dbm.create_user(username, password_hash)?;
        Ok(QueryResult::OK)
    }
    pub fn api_user_login(
        dbm: &mut DatabaseManager,
        username: &str,
        password_hash: &str,
    ) -> DBMError {
        dbm.login(username, password_hash)?;
        Ok(QueryResult::OK)
    }
    pub fn api_user_change_password(
        dbm: &mut DatabaseManager,
        old_password: Option<&str>,
        new_password: &str,
    ) -> DBMError {
        dbm.change_password(old_password, new_password)?;
        Ok(QueryResult::OK)
    }
    pub fn api_user_rename(
        dbm: &mut DatabaseManager,
        old_username: &str,
        new_username: &str,
    ) -> DBMError {
        dbm.rename_user(old_username, new_username)?;
        Ok(QueryResult::OK)
    }
    pub fn api_user_drop(dbm: &mut DatabaseManager, username: &str) -> DBMError {
        dbm.drop_user(username)?;
        Ok(QueryResult::OK)
    }

    // -- TABLE
    pub fn api_table_create(
        &mut self,
        table_name: &str,
        raw_columns: Vec<(String, DataType, Vec<ColumnConstraint>)>,
    ) -> DBMError {
        let db_mut = self.dbm.active_database_mut()?;
        db_mut.create_table(table_name, raw_columns)?;
        Ok(QueryResult::OK)
    }
    pub fn api_table_rename(&mut self, old_name: &str, new_name: &str) -> DBMError {
        let db_mut = self.dbm.active_database_mut()?;
        db_mut.rename_table(old_name, new_name)?;
        Ok(QueryResult::OK)
    }
    pub fn api_table_drop(&mut self, table_name: &str) -> DBMError {
        let db_mut = self.dbm.active_database_mut()?;
        db_mut.drop_table(table_name)?;
        Ok(QueryResult::OK)
    }
    pub fn api_table_show(dbm: &DatabaseManager) -> DBMError {
        let (schema, rows) = dbm.active_database_ref()?.show_tables()?;
        Ok(QueryResult::Dql { schema, rows })
    }
    pub fn api_table_describe(dbm: &DatabaseManager, table_name: &str) -> DBMError {
        let db_ref = dbm.active_database_ref()?;
        db_ref.describe_table(table_name)
    }

    // -- DDL
    pub fn api_column_add(
        &mut self,
        table_name: &str,
        raw_columns: Vec<(String, DataType, Vec<ColumnConstraint>, ColumnPosition)>,
    ) -> DBMError {
        let db_mut = self.dbm.active_database_mut()?;
        db_mut.add_columns(table_name, raw_columns)?;
        Ok(QueryResult::OK)
    }
    pub fn api_column_drop(&mut self, table_name: &str, col_name: &str) -> DBMError {
        let db_mut = self.dbm.active_database_mut()?;
        db_mut.drop_column(table_name, col_name)?;
        Ok(QueryResult::OK)
    }
    pub fn api_column_rename(
        &mut self,
        table_name: &str,
        old_name: &str,
        new_name: &str,
    ) -> DBMError {
        let db_mut = self.dbm.active_database_mut()?;
        db_mut.rename_column(table_name, old_name, new_name)?;
        Ok(QueryResult::OK)
    }
    pub fn api_column_modify_type(
        &mut self,
        table_name: &str,
        col_name: &str,
        new_type: DataType,
    ) -> DBMError {
        let db_mut = self.dbm.active_database_mut()?;
        db_mut.modify_column_type(table_name, col_name, new_type)?;
        Ok(QueryResult::OK)
    }
    pub fn api_column_constraint_add(
        &mut self,
        table_name: &str,
        col_name: &str,
        constraint: ColumnConstraint,
    ) -> DBMError {
        let db_mut = self.dbm.active_database_mut()?;
        db_mut.add_column_constraint(table_name, col_name, constraint)?;
        Ok(QueryResult::OK)
    }
    pub fn api_column_constraint_drop(
        &mut self,
        table_name: &str,
        col_name: &str,
        constraint: ColumnConstraint,
    ) -> DBMError {
        let db_mut = self.dbm.active_database_mut()?;
        db_mut.drop_column_constraint(table_name, col_name, constraint)?;
        Ok(QueryResult::OK)
    }
    pub fn api_column_set_default(
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
    pub fn api_row_insert(&mut self, table_name: &str, rows: Vec<Vec<ValueType>>) -> DBMError {
        let db_mut = self.dbm.active_database_mut()?;
        db_mut.insert(table_name, rows)
    }
    pub fn api_row_update(
        &mut self,
        table_name: &str,
        assignments: HashMap<String, Expr>,
        predicate: Option<Expr>,
    ) -> DBMError {
        let db_mut = self.dbm.active_database_mut()?;
        db_mut.update(table_name, assignments, predicate)
    }
    pub fn api_row_delete(&mut self, table_name: &str, predicate: Option<Expr>) -> DBMError {
        let db_mut = self.dbm.active_database_mut()?;
        db_mut.delete(table_name, predicate)
    }

    // -- DQL
    pub fn api_select(&mut self, table_name: &str, statements: SelectStmt) -> DBMError {
        let db_mut = self.dbm.active_database_mut()?;
        db_mut.select(table_name, statements)
    }
}
