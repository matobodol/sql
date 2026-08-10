use crate::{
    CommandAction, Database, DomainError, QueryResult, TableId, TableStorage,
    catalog::CatalogStore,
    command::{execute_alter_table, execute_dml_action, execute_table_action},
    dql_action::execute_select,
    table_action::{execute_describe_table, execute_show_tables},
};
use std::collections::HashMap;

#[derive(Debug)]
pub struct SystemCatalog {
    current_user: Option<String>,
    active_db: Option<String>,
    catalog: CatalogStore,
    databases: HashMap<(String, String), Database>,
}

impl SystemCatalog {
    pub fn new() -> Self {
        Self {
            current_user: Some("root".to_string()),
            catalog: CatalogStore::new(),
            databases: HashMap::new(),
            active_db: None,
        }
    }

    #[inline]
    fn current_username(&self) -> Result<&str, DomainError> {
        self.current_user
            .as_deref()
            .ok_or_else(|| DomainError::catalog("Tidak ada user yang sedang login."))
    }

    pub fn create_database(&mut self, db_name: &str) -> Result<(), DomainError> {
        let username = self.current_username()?;
        let user_key = username.to_lowercase();
        let db_key = db_name.to_lowercase();

        let composite_key = (user_key, db_key);
        if self.databases.contains_key(&composite_key) {
            return Err(DomainError::DatabaseAlreadyExists(db_name.into()));
        }

        let db = Database::new(db_name);
        self.databases.insert(composite_key, db);
        Ok(())
    }

    pub fn use_database(&mut self, db_name: &str) -> Result<(), DomainError> {
        let username = &self.current_username()?.to_lowercase();
        let user_key = username.to_lowercase();
        let db_key = db_name.to_lowercase();

        let is_admin = self.catalog.users_mut().is_admin(username)?;

        let target_key = if is_admin {
            self.databases
                .keys()
                .find(|(_, d)| d == &db_key)
                .cloned()
                .ok_or_else(|| DomainError::DatabaseNotFound(db_name.into()))?
        } else {
            let composite_key = (user_key, db_key);
            if !self.databases.contains_key(&composite_key) {
                return Err(DomainError::DatabaseNotFound(db_name.into()));
            }
            composite_key
        };

        self.active_db = Some(target_key.1);
        Ok(())
    }

    pub fn drop_database(&mut self, db_name: &str) -> Result<(), DomainError> {
        let username = &self.current_username()?.to_lowercase();
        let user_key = username.to_lowercase();
        let db_key = db_name.to_lowercase();

        let is_admin = self.catalog.users_mut().is_admin(username)?;

        let target_key = if is_admin {
            self.databases
                .keys()
                .find(|(_, d)| d == &db_key)
                .cloned()
                .ok_or_else(|| DomainError::DatabaseNotFound(db_name.into()))?
        } else {
            (user_key, db_key)
        };

        if self.databases.remove(&target_key).is_none() {
            return Err(DomainError::DatabaseNotFound(db_name.into()));
        }

        if self.active_db.as_deref() == Some(&target_key.1) {
            self.active_db = None;
        }

        Ok(())
    }

    pub fn active_database_mut(&mut self) -> Result<&mut Database, DomainError> {
        let username = &self.current_username()?.to_string();
        let user_key = username.to_string();

        let active_db_name = self
            .active_db
            .as_ref()
            .ok_or(DomainError::NoActiveDatabase)?;
        let db_key = active_db_name.to_string();

        let is_admin = self.catalog.users_mut().is_admin(username)?;

        let target_key = if is_admin {
            self.databases
                .keys()
                .find(|(_, d)| d == &db_key)
                .cloned()
                .ok_or_else(|| DomainError::DatabaseNotFound(active_db_name.clone().into()))?
        } else {
            (user_key, db_key)
        };

        self.databases
            .get_mut(&target_key)
            .ok_or_else(|| DomainError::DatabaseNotFound(active_db_name.clone().into()))
    }

    pub fn active_db_name(&self) -> Option<&str> {
        self.active_db.as_deref()
    }

    pub fn table_mut(&mut self, table_name: &str) -> Result<&mut TableStorage, DomainError> {
        let table_id = self.catalog.get_table_id(table_name)?;
        let db = self.active_database_mut()?;

        db.table_mut((table_name, &table_id))
    }
}

impl SystemCatalog {
    pub fn create_user(&mut self, username: &str, password_hash: &str) -> Result<(), DomainError> {
        let current = &self.current_username()?.to_lowercase();
        let is_admin = self.catalog.users_mut().is_admin(current)?;

        if !is_admin {
            return Err(DomainError::catalog(
                "Hanya admin yang dapat membuat user baru.",
            ));
        }

        self.catalog
            .users_mut()
            .create_user(username, password_hash)
    }

    pub fn login(&mut self, username: &str, password: &str) -> Result<(), DomainError> {
        self.catalog.users_mut().authenticate(username, password)?;
        self.current_user = Some(username.to_string());
        self.active_db = None;
        Ok(())
    }

    pub fn change_password(
        &mut self,
        old_password: Option<&str>,
        new_password: &str,
    ) -> Result<(), DomainError> {
        let username = self.current_username()?.to_string();
        self.catalog
            .users_mut()
            .change_password(&username, old_password, new_password)
    }
}

impl SystemCatalog {
    fn catalog_and_tables_mut(
        &mut self,
    ) -> Result<(&mut CatalogStore, &mut HashMap<TableId, TableStorage>), DomainError> {
        let username = &self.current_username()?.to_string();
        let user_key = username.to_string();

        let active_db_name = self
            .active_db
            .as_ref()
            .ok_or(DomainError::NoActiveDatabase)?
            .clone();
        let db_key = active_db_name.to_string();

        // Peminjaman catalog sementara untuk cek admin (borrow berakhir di sini)
        let is_admin = self.catalog.users_mut().is_admin(username)?;

        let target_key = if is_admin {
            self.databases
                .keys()
                .find(|(_, d)| d == &db_key)
                .cloned()
                .ok_or_else(|| DomainError::DatabaseNotFound(active_db_name.clone().into()))?
        } else {
            (user_key, db_key)
        };

        // Ambil referensi mutabel ke database dari self.databases
        let database = self
            .databases
            .get_mut(&target_key)
            .ok_or_else(|| DomainError::DatabaseNotFound(active_db_name.into()))?;

        let tables = database.tables_mut();

        // Karena self.catalog dan database (bagian dari self.databases) adalah
        // field yang terpisah (disjoint), Rust mengizinkan keduanya dipinjam bersamaan.
        Ok((&mut self.catalog, tables))
    }

    pub fn execute(&mut self, action: CommandAction) -> Result<QueryResult, DomainError> {
        let (catalog, tables) = self.catalog_and_tables_mut()?;
        let list_table = catalog.list_tables();

        match action {
            // -- DDL ACTION --
            CommandAction::TableAction { actions } => {
                execute_table_action(catalog, tables, actions)?;
                Ok(QueryResult::OK)
            }
            CommandAction::AlterTable {
                table_name,
                actions,
            } => {
                execute_alter_table(catalog, tables, &table_name, actions)?;
                Ok(QueryResult::OK)
            }

            // -- DML ACTION --
            CommandAction::DmlAction { table_name, action } => {
                let table_id = catalog.get_table_id(&table_name)?;
                let table_storage = tables
                    .get_mut(&table_id)
                    .ok_or_else(|| DomainError::TableNotFound(table_name.into()))?;

                execute_dml_action(catalog, table_storage, table_id, action)
            }

            // -- DQL ACTION --
            CommandAction::Select {
                table_name,
                statements,
            } => {
                let table_id = catalog.get_table_id(&table_name)?;
                let table_storage = tables
                    .get(&table_id)
                    .ok_or_else(|| DomainError::TableNotFound(table_name.into()))?;

                execute_select(catalog, table_storage, table_id, statements)
            }
            CommandAction::ShowTables => execute_show_tables(&list_table),
            CommandAction::DescribeTable { table_name } => {
                let table_id = catalog.get_table_id(&table_name)?;
                let schema = catalog.get_schema(table_id)?;
                execute_describe_table(schema.columns())
            }
        }
    }
}
