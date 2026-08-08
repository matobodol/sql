use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    Column, ColumnConstraint, ColumnId, DataType, DomainError, Schema, TableId,
    catalog::id::IdGenerator,
};

#[derive(Debug, Default)]
pub struct CatalogStore {
    id_generator: IdGenerator,
    table_name_to_id: HashMap<String, TableId>,
    table_id_to_name: HashMap<TableId, String>,
    table_schemas: HashMap<TableId, Arc<Schema>>,
    column_name_to_id: HashMap<(TableId, String), ColumnId>,
    // Mengubah kunci pemetaan menjadi tuple (TableId, ColumnId) agar unik per tabel
    column_id_to_name: HashMap<(TableId, ColumnId), String>,
    // Menyimpan penomoran kolom berikutnya untuk setiap tabel (dimulai dari 1)
    table_next_column_id: HashMap<TableId, u32>,
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
    pub fn get_table_id(&self, name: &str) -> Option<TableId> {
        self.table_name_to_id.get(&name.to_lowercase()).copied()
    }

    #[inline]
    pub fn get_column_id(&self, table_id: TableId, name: &str) -> Option<ColumnId> {
        self.column_name_to_id
            .get(&(table_id, name.to_lowercase()))
            .copied()
    }

    pub fn register_table(&mut self, name: &str) -> Result<TableId, DomainError> {
        let name_lower = name.to_lowercase();
        if self.table_name_to_id.contains_key(&name_lower) {
            return Err(DomainError::TableAlreadyExists(Arc::from(name)));
        }

        let new_id = self.id_generator.next_table_id();

        self.table_name_to_id.insert(name_lower, new_id);
        self.table_id_to_name.insert(new_id, name.to_string());
        self.table_schemas
            .insert(new_id, Arc::new(Schema::default()));

        // Inisialisasi counter kolom untuk tabel baru mulai dari 1
        self.table_next_column_id.insert(new_id, 1);

        Ok(new_id)
    }

    pub fn register_column(
        &mut self,
        table_id: TableId,
        name: &str,
        sql_type: DataType,
        constraints: Vec<ColumnConstraint>,
    ) -> Result<ColumnId, DomainError> {
        if let Some(col_id) = self.get_column_id(table_id, name) {
            return Ok(col_id);
        }

        let schema_arc = self.table_schemas.get(&table_id).ok_or_else(|| {
            DomainError::TableNotFound(Arc::from(format!("TableId {:?} tidak ditemukan", table_id)))
        })?;

        // Mengambil dan menaikkan ID kolom khusus untuk tabel ini
        let next_id_u32 = self
            .table_next_column_id
            .get_mut(&table_id)
            .ok_or_else(|| {
                DomainError::TableNotFound(Arc::from(format!(
                    "TableId {:?} tidak ditemukan",
                    table_id
                )))
            })?;
        let col_id = ColumnId(*next_id_u32);
        *next_id_u32 += 1;

        let new_col_def = Column::with_constraints(col_id, name, sql_type, constraints);

        let mut new_schema = (**schema_arc).clone();
        new_schema.add_columns(vec![new_col_def])?;

        self.column_name_to_id
            .insert((table_id, name.to_lowercase()), col_id);
        self.column_id_to_name
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
        if let Some(col_id) = self.get_column_id(table_id, name) {
            return Ok(col_id);
        }

        let schema_arc = self.table_schemas.get(&table_id).ok_or_else(|| {
            DomainError::TableNotFound(Arc::from(format!("TableId {:?} tidak ditemukan", table_id)))
        })?;

        let next_id_u32 = self
            .table_next_column_id
            .get_mut(&table_id)
            .ok_or_else(|| {
                DomainError::TableNotFound(Arc::from(format!(
                    "TableId {:?} tidak ditemukan",
                    table_id
                )))
            })?;
        let col_id = ColumnId(*next_id_u32);
        *next_id_u32 += 1;

        let new_col_def = Column::with_constraints(col_id, name, sql_type, constraints);

        let mut new_schema = (**schema_arc).clone();
        new_schema.insert_column(index, new_col_def)?;

        self.column_name_to_id
            .insert((table_id, name.to_lowercase()), col_id);
        self.column_id_to_name
            .insert((table_id, col_id), name.to_string());
        self.table_schemas.insert(table_id, Arc::new(new_schema));

        Ok(col_id)
    }

    pub fn unregister_column(
        &mut self,
        table_id: TableId,
        col_name: &str,
    ) -> Result<ColumnId, DomainError> {
        let col_id = self.get_column_id(table_id, col_name).ok_or_else(|| {
            DomainError::eval_error(format!("Kolom '{col_name}' tidak ditemukan"))
        })?;

        let schema_arc = self.table_schemas.get(&table_id).ok_or_else(|| {
            DomainError::TableNotFound(Arc::from(format!("TableId {:?} tidak ditemukan", table_id)))
        })?;

        let mut new_schema = (**schema_arc).clone();
        new_schema.remove_column(col_id)?;

        self.column_name_to_id
            .remove(&(table_id, col_name.to_lowercase()));
        self.column_id_to_name.remove(&(table_id, col_id));
        self.table_schemas.insert(table_id, Arc::new(new_schema));

        Ok(col_id)
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

        if !old_n.eq_ignore_ascii_case(&new_n) {
            self.column_name_to_id
                .remove(&(table_id, old_n.to_lowercase()));
            self.column_name_to_id
                .insert((table_id, new_n.to_lowercase()), col_id);
            self.column_id_to_name.insert((table_id, col_id), new_n);
        }

        self.table_schemas.insert(table_id, Arc::new(new_schema));
        Ok(())
    }

    pub fn rename_table(&mut self, old_name: &str, new_name: &str) -> Result<TableId, DomainError> {
        let old_name_lower = old_name.to_lowercase();
        let new_name_lower = new_name.to_lowercase();

        if self.table_name_to_id.contains_key(&new_name_lower) {
            return Err(DomainError::TableAlreadyExists(Arc::from(new_name)));
        }

        let table_id = self
            .table_name_to_id
            .remove(&old_name_lower)
            .ok_or_else(|| DomainError::TableNotFound(Arc::from(old_name)))?;

        self.table_name_to_id.insert(new_name_lower, table_id);
        self.table_id_to_name.insert(table_id, new_name.to_string());

        Ok(table_id)
    }

    #[inline]
    pub fn get_schema(&self, table_id: TableId) -> Result<Arc<Schema>, DomainError> {
        self.table_schemas.get(&table_id).cloned().ok_or_else(|| {
            DomainError::TableNotFound(Arc::from(format!("TableId {:?} tidak ditemukan", table_id)))
        })
    }

    #[inline]
    pub fn get_schema_columns(&self, table_id: TableId) -> Option<Arc<[Column]>> {
        self.table_schemas
            .get(&table_id)
            .map(|s| Arc::from(s.columns()))
    }

    #[inline]
    pub fn list_tables(&self) -> Vec<String> {
        self.table_id_to_name.values().cloned().collect()
    }

    pub fn unregister_table(&mut self, name: &str) -> Result<TableId, DomainError> {
        let table_id = self
            .get_table_id(name)
            .ok_or_else(|| DomainError::TableNotFound(Arc::from(name)))?;

        let name_lower = name.to_lowercase();
        self.table_name_to_id.remove(&name_lower);
        self.table_id_to_name.remove(&table_id);
        self.table_next_column_id.remove(&table_id);

        if let Some(schema) = self.table_schemas.remove(&table_id) {
            for col in schema.columns() {
                self.column_id_to_name.remove(&(table_id, col.id));
                self.column_name_to_id
                    .remove(&(table_id, col.name.to_lowercase()));
            }
        }

        Ok(table_id)
    }

    pub fn update_schema(&mut self, table_id: TableId, new_schema: Schema) {
        self.table_schemas.insert(table_id, Arc::new(new_schema));
    }
}
