use crate::domain::DomainError;
use crate::domain::id::{ColumnId, IdGenerator, TableId};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct SymbolRegistry {
    column_id_gen: IdGenerator,
    table_id_gen: IdGenerator,

    // Mapping Column: Name <-> ID
    col_name_to_id: HashMap<String, ColumnId>,
    col_id_to_name: HashMap<ColumnId, String>,

    // Mapping Table: Name <-> ID
    table_name_to_id: HashMap<String, TableId>,
    table_id_to_name: HashMap<TableId, String>,
}

impl SymbolRegistry {
    pub fn new() -> Self {
        Self {
            column_id_gen: IdGenerator::new(1),
            table_id_gen: IdGenerator::new(1),
            col_name_to_id: HashMap::new(),
            col_id_to_name: HashMap::new(),
            table_name_to_id: HashMap::new(),
            table_id_to_name: HashMap::new(),
        }
    }

    // --- COLUMN REGISTRY ---

    /// Mendaftarkan nama kolom baru (jika belum ada) dan mengembalikan ColumnId-nya
    pub fn register_column(&mut self, name: &str) -> ColumnId {
        let name_lower = name.to_lowercase();
        if let Some(&id) = self.col_name_to_id.get(&name_lower) {
            return id;
        }

        let new_id = ColumnId(self.column_id_gen.next_id());
        self.col_name_to_id.insert(name_lower, new_id);
        self.col_id_to_name.insert(new_id, name.to_string());
        new_id
    }

    pub fn get_column_id(&self, name: &str) -> Option<ColumnId> {
        self.col_name_to_id.get(&name.to_lowercase()).copied()
    }

    pub fn get_column_name(&self, id: ColumnId) -> Option<&str> {
        self.col_id_to_name.get(&id).map(|s| s.as_str())
    }

    /// RENAME COLUMN: Operasi O(1) tanpa menyentuh data tabel
    pub fn rename_column(&mut self, old_name: &str, new_name: &str) -> Result<(), DomainError> {
        let old_lower = old_name.to_lowercase();
        let new_lower = new_name.to_lowercase();

        let id = self.col_name_to_id.remove(&old_lower).ok_or_else(|| {
            DomainError::EvaluationError(format!(
                "Kolom '{}' tidak ditemukan di Registry",
                old_name
            ))
        })?;

        if self.col_name_to_id.contains_key(&new_lower) {
            // Restore jika bentrok
            self.col_name_to_id.insert(old_lower, id);
            return Err(DomainError::EvaluationError(format!(
                "Nama kolom baru '{}' sudah terpakai di Registry",
                new_name
            )));
        }

        self.col_name_to_id.insert(new_lower, id);
        self.col_id_to_name.insert(id, new_name.to_string());
        Ok(())
    }

    // --- TABLE REGISTRY ---

    pub fn register_table(&mut self, name: &str) -> Result<TableId, DomainError> {
        let name_lower = name.to_lowercase();
        if self.table_name_to_id.contains_key(&name_lower) {
            return Err(DomainError::TableAlreadyExists(name.to_string()));
        }

        let new_id = TableId(self.table_id_gen.next_id());
        self.table_name_to_id.insert(name_lower, new_id);
        self.table_id_to_name.insert(new_id, name.to_string());
        Ok(new_id)
    }

    pub fn get_table_id(&self, name: &str) -> Option<TableId> {
        self.table_name_to_id.get(&name.to_lowercase()).copied()
    }

    pub fn get_table_name(&self, id: TableId) -> Option<&str> {
        self.table_id_to_name.get(&id).map(|s| s.as_str())
    }
}
