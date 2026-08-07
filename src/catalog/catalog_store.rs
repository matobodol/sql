use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    Column, ColumnConstraint, DomainError, Schema, SqlType,
    id::{ColumnId, IdGenerator, TableId},
};

#[derive(Debug, Default)]
pub struct CatalogStore {
    id_generator: IdGenerator,
    table_name_to_id: HashMap<String, TableId>,
    table_id_to_name: HashMap<TableId, String>,
    table_schemas: HashMap<TableId, Arc<[Column]>>,
    column_name_to_id: HashMap<(TableId, String), ColumnId>,
    column_id_to_name: HashMap<ColumnId, String>,
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

    pub fn register_table(&mut self, name: &str) -> Result<TableId, DomainError> {
        let name_lower = name.to_lowercase();
        if self.table_name_to_id.contains_key(&name_lower) {
            return Err(DomainError::TableAlreadyExists(Arc::from(name)));
        }

        let new_id = self.id_generator.next_table_id();

        self.table_name_to_id.insert(name_lower, new_id);
        self.table_id_to_name.insert(new_id, name.to_string());
        self.table_schemas.insert(new_id, Arc::from([]));

        Ok(new_id)
    }

    pub fn register_column(
        &mut self,
        table_id: TableId,
        name: &str,
        sql_type: SqlType,
        constraints: Vec<ColumnConstraint>,
    ) -> Result<ColumnId, DomainError> {
        let name_lower = name.to_lowercase();
        let key = (table_id, name_lower.clone());

        if let Some(&col_id) = self.column_name_to_id.get(&key) {
            return Ok(col_id);
        }

        let schema_arc = self.table_schemas.get(&table_id).ok_or_else(|| {
            DomainError::TableNotFound(Arc::from(format!("TableId {:?} tidak ditemukan", table_id)))
        })?;

        let col_id = self.id_generator.next_column_id();
        let new_col_def = Column::with_constraints(col_id, name, sql_type, constraints);

        let mut vec_cols = schema_arc.to_vec();
        vec_cols.push(new_col_def);

        Schema::validate_schema_columns(&vec_cols)?;

        self.column_name_to_id.insert(key, col_id);
        self.column_id_to_name.insert(col_id, name.to_string());
        self.table_schemas.insert(table_id, Arc::from(vec_cols));

        Ok(col_id)
    }

    #[inline]
    pub fn get_schema_columns(&self, table_id: TableId) -> Option<Arc<[Column]>> {
        self.table_schemas.get(&table_id).cloned()
    }

    pub fn get_schema(&self, table_id: TableId) -> Result<Schema, DomainError> {
        let cols_arc = self.get_schema_columns(table_id).ok_or_else(|| {
            DomainError::TableNotFound(Arc::from(format!("TableId {:?} tidak ditemukan", table_id)))
        })?;
        Schema::new(cols_arc.to_vec())
    }

    #[inline]
    pub fn list_tables(&self) -> Vec<String> {
        self.table_id_to_name.values().cloned().collect()
    }

    #[inline]
    pub fn get_table_id(&self, name: &str) -> Option<TableId> {
        self.table_name_to_id.get(&name.to_lowercase()).copied()
    }

    #[inline]
    pub fn get_column_id(&self, table_id: TableId, name: &str) -> Option<ColumnId> {
        self.column_name_to_id
            .get(&(table_id, name.to_lowercase()))
            .copied()
    }

    /// Mengganti nama tabel secara in-place tanpa merusak ID atau B-Tree Index
    pub fn rename_table(&mut self, old_name: &str, new_name: &str) -> Result<TableId, DomainError> {
        let old_lower = old_name.to_lowercase();
        let new_lower = new_name.to_lowercase();

        if self.table_name_to_id.contains_key(&new_lower) {
            return Err(DomainError::TableAlreadyExists(Arc::from(new_name)));
        }

        let table_id = self
            .table_name_to_id
            .remove(&old_lower)
            .ok_or_else(|| DomainError::TableNotFound(Arc::from(old_name)))?;

        self.table_id_to_name.insert(table_id, new_name.to_string());
        self.table_name_to_id.insert(new_lower, table_id);

        Ok(table_id)
    }

    pub fn unregister_table(&mut self, name: &str) -> Result<TableId, DomainError> {
        let name_lower = name.to_lowercase();
        let table_id = self
            .table_name_to_id
            .remove(&name_lower)
            .ok_or_else(|| DomainError::TableNotFound(Arc::from(name)))?;

        self.table_id_to_name.remove(&table_id);
        self.table_schemas.remove(&table_id);

        // Mencegah Memory Leak: hapus juga dari column_id_to_name
        let mut cols_to_remove = Vec::new();
        self.column_name_to_id.retain(|(t_id, _), col_id| {
            if *t_id == table_id {
                cols_to_remove.push(*col_id);
                false
            } else {
                true
            }
        });

        for col_id in cols_to_remove {
            self.column_id_to_name.remove(&col_id);
        }

        Ok(table_id)
    }

    pub fn unregister_column(
        &mut self,
        table_id: TableId,
        col_name: &str,
    ) -> Result<(), DomainError> {
        let name_lower = col_name.to_lowercase();
        let key = (table_id, name_lower);

        let col_id = self
            .column_name_to_id
            .remove(&key)
            .ok_or_else(|| DomainError::ColumnNotFound(Arc::from(col_name)))?;

        self.column_id_to_name.remove(&col_id);

        if let Some(schema_arc) = self.table_schemas.get_mut(&table_id) {
            let mut vec_cols = schema_arc.to_vec();
            vec_cols.retain(|c| c.id != col_id);

            Schema::validate_schema_columns(&vec_cols)?;
            *schema_arc = Arc::from(vec_cols);
        }

        Ok(())
    }

    pub fn mutate_column<F>(
        &mut self,
        table_id: TableId,
        col_id: ColumnId,
        f: F,
    ) -> Result<(), DomainError>
    where
        F: FnOnce(&mut Column),
    {
        if let Some(schema_arc) = self.table_schemas.get_mut(&table_id) {
            let mut vec_cols = schema_arc.to_vec();
            let col = vec_cols
                .iter_mut()
                .find(|c| c.id == col_id)
                .ok_or_else(|| {
                    DomainError::eval_error("Kolom tidak ditemukan pada skema katalog")
                })?;
            f(col);

            Schema::validate_schema_columns(&vec_cols)?;
            *schema_arc = Arc::from(vec_cols);
        }
        Ok(())
    }
}
