use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{
    Column, ColumnConstraint, ColumnId, DataType, DomainError, Schema, TableId,
    catalog::id::IdGenerator,
};

pub const DEFAULT_ADMIN: &str = "root";
pub const BASE_PATH: &str = ".data";
pub const GLOBAL_USER_PATH: &str = ".data/GLOBAL_USER.bin";
pub const EXT_AUTO_INC: &str = ".auto_inc";
pub const EXT_INDEX_REGISTRY: &str = ".index";
pub const METADATA: &str = "METADATA.bin";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CatalogStore {
    id_generator: IdGenerator,

    // -- TABLE META --
    table_to_id: HashMap<String, TableId>,
    table_to_name: HashMap<TableId, String>,
    table_schemas: HashMap<TableId, Arc<Schema>>,

    // -- COLUMN META --
    column_to_id: HashMap<(TableId, String), ColumnId>,
    column_to_name: HashMap<(TableId, ColumnId), String>,
}

impl CatalogStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_generator(id_generator: IdGenerator) -> Self {
        Self {
            id_generator,
            ..Default::default()
        }
    }

    #[inline]
    pub fn get_table_id(&self, name: &str) -> Result<TableId, DomainError> {
        self.table_to_id
            .get(name)
            .copied()
            .ok_or_else(|| DomainError::TableNotFound(Arc::from(name)))
    }

    // Tambahkan di impl CatalogStore (catalog_store.rs)
    #[inline]
    pub fn get_table_name(&self, table_id: TableId) -> Result<String, DomainError> {
        self.table_to_name.get(&table_id).cloned().ok_or_else(|| {
            DomainError::TableNotFound(Arc::from(format!("TableId {:?} tidak ditemukan", table_id)))
        })
    }

    #[inline]
    pub fn get_column_id(&self, table_id: TableId, name: &str) -> Result<ColumnId, DomainError> {
        self.column_to_id
            .get(&(table_id, name.to_string()))
            .copied()
            .ok_or_else(|| DomainError::ColumnNotFound(Arc::from(name)))
    }

    #[inline]
    pub fn get_column_name(
        &self,
        table_id: TableId,
        col_id: ColumnId,
    ) -> Result<String, DomainError> {
        self.column_to_name
            .get(&(table_id, col_id))
            .map(|n| n.clone())
            .ok_or_else(|| DomainError::eval_error("column id tidak ditemukan"))
    }

    pub fn register_table(&mut self, name: &str) -> Result<TableId, DomainError> {
        if self.table_to_id.contains_key(name) {
            return Err(DomainError::TableAlreadyExists(Arc::from(name)));
        }

        let new_id = self.id_generator.next_table_id();

        self.table_to_id.insert(name.to_string(), new_id);
        self.table_to_name.insert(new_id, name.to_string());
        self.table_schemas
            .insert(new_id, Arc::new(Schema::default()));

        Ok(new_id)
    }

    pub fn register_column(
        &mut self,
        table_id: TableId,
        name: &str,
        sql_type: DataType,
        constraints: Vec<ColumnConstraint>,
    ) -> Result<ColumnId, DomainError> {
        if let Ok(col_id) = self.get_column_id(table_id, name) {
            return Ok(col_id);
        }

        let schema_arc = self.table_schemas.get(&table_id).ok_or_else(|| {
            DomainError::TableNotFound(Arc::from(format!("TableId {:?} tidak ditemukan", table_id)))
        })?;

        let col_id = self.id_generator.next_column_id(table_id);
        let new_col_def = Column::with_constraints(col_id, name, sql_type, constraints);

        let mut new_schema = (**schema_arc).clone();
        new_schema.add_columns(vec![new_col_def])?;

        self.column_to_id
            .insert((table_id, name.to_string()), col_id);
        self.column_to_name
            .insert((table_id, col_id), name.to_string());
        self.table_schemas.insert(table_id, Arc::new(new_schema));

        Ok(col_id)
    }

    pub fn register_column_at(
        &mut self,
        table_id: TableId,
        name: &str,
        sql_type: DataType,
        constraints: Vec<ColumnConstraint>,
        index: usize,
    ) -> Result<ColumnId, DomainError> {
        if let Ok(col_id) = self.get_column_id(table_id, name) {
            return Ok(col_id);
        }

        let schema_arc = self.table_schemas.get(&table_id).ok_or_else(|| {
            DomainError::TableNotFound(Arc::from(format!("TableId {:?} tidak ditemukan", table_id)))
        })?;

        let col_id = self.id_generator.next_column_id(table_id);
        let new_col_def = Column::with_constraints(col_id, name, sql_type, constraints);

        let mut new_schema = (**schema_arc).clone();
        new_schema.insert_column(index, new_col_def)?;

        self.column_to_id
            .insert((table_id, name.to_string()), col_id);
        self.column_to_name
            .insert((table_id, col_id), name.to_string());

        self.table_schemas.insert(table_id, Arc::new(new_schema));

        Ok(col_id)
    }

    pub fn unregister_column(
        &mut self,
        table_id: TableId,
        col_name: &str,
    ) -> Result<ColumnId, DomainError> {
        let col_id = self.get_column_id(table_id, col_name)?;

        let schema_arc = self.table_schemas.get(&table_id).ok_or_else(|| {
            DomainError::TableNotFound(Arc::from(format!("TableId {:?} tidak ditemukan", table_id)))
        })?;

        let mut new_schema = (**schema_arc).clone();
        new_schema.remove_column(col_id)?;

        self.column_to_id.remove(&(table_id, col_name.to_string()));
        self.column_to_name.remove(&(table_id, col_id));

        // Delegasi aturan reset ID kolom ke generator
        self.id_generator
            .reset_column_counter_if_empty(table_id, new_schema.columns().is_empty());

        self.table_schemas.insert(table_id, Arc::new(new_schema));

        Ok(col_id)
    }

    pub fn unregister_table(&mut self, name: &str) -> Result<TableId, DomainError> {
        let table_id = self.get_table_id(name)?;

        self.table_to_id.remove(name);
        self.table_to_name.remove(&table_id);

        // Bersihkan tracking counter kolom tabel tersebut dari generator
        self.id_generator.remove_table_counter(table_id);

        if let Some(schema) = self.table_schemas.remove(&table_id) {
            for col in schema.columns() {
                self.column_to_name.remove(&(table_id, col.id));
                self.column_to_id.remove(&(table_id, col.name.clone()));
            }
        }

        // Delegasi aturan reset ID tabel ke generator jika sudah habis total
        self.id_generator
            .reset_table_if_empty(self.table_to_name.is_empty());

        Ok(table_id)
    }

    pub fn mutate_column<F>(
        &mut self,
        table_id: TableId,
        col_id: ColumnId,
        mut mutator: F,
    ) -> Result<(), DomainError>
    where
        F: FnMut(&mut Column),
    {
        let schema_arc = self.table_schemas.get(&table_id).ok_or_else(|| {
            DomainError::TableNotFound(Arc::from(format!("TableId {:?} tidak ditemukan", table_id)))
        })?;

        let mut new_schema = (**schema_arc).clone();
        let mut old_name = None;
        let mut new_name = None;

        for col in new_schema.columns_mut() {
            if col.id == col_id {
                old_name = Some(col.name.clone());
                mutator(col);
                new_name = Some(col.name.clone());
                break;
            }
        }

        let (old_n, new_n) = match (old_name, new_name) {
            (Some(o), Some(n)) => (o, n),
            _ => {
                return Err(DomainError::eval_error(format!(
                    "ColumnId {:?} tidak ditemukan di skema",
                    col_id
                )));
            }
        };

        if old_n != new_n {
            self.column_to_id.remove(&(table_id, old_n));
            self.column_to_id.insert((table_id, new_n.clone()), col_id);
            self.column_to_name.insert((table_id, col_id), new_n);
        }

        self.table_schemas.insert(table_id, Arc::new(new_schema));
        Ok(())
    }

    pub fn rename_table(&mut self, old_name: &str, new_name: &str) -> Result<TableId, DomainError> {
        if self.table_to_id.contains_key(new_name) {
            return Err(DomainError::TableAlreadyExists(Arc::from(new_name)));
        }

        let table_id = self
            .table_to_id
            .remove(old_name)
            .ok_or_else(|| DomainError::TableNotFound(Arc::from(old_name)))?;

        self.table_to_id.insert(new_name.to_string(), table_id);
        self.table_to_name.insert(table_id, new_name.to_string());

        Ok(table_id)
    }

    #[inline]
    pub fn get_schema(&self, table_id: TableId) -> Result<Arc<Schema>, DomainError> {
        self.table_schemas.get(&table_id).cloned().ok_or_else(|| {
            DomainError::TableNotFound(Arc::from(format!("TableId {:?} tidak ditemukan", table_id)))
        })
    }

    #[inline]
    pub fn get_schema_columns(&self, table_id: TableId) -> Result<Arc<[Column]>, DomainError> {
        let schema_arc = self.table_schemas.get(&table_id).ok_or_else(|| {
            DomainError::TableNotFound(Arc::from(format!("TableId {:?} tidak ditemukan", table_id)))
        })?;
        Ok(Arc::from(schema_arc.columns()))
    }

    #[inline]
    pub fn list_tables(&self) -> Vec<String> {
        self.table_to_name.values().cloned().collect()
    }

    pub fn update_schema(&mut self, table_id: TableId, new_schema: Schema) {
        self.table_schemas.insert(table_id, Arc::new(new_schema));
    }
}
