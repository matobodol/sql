use serde::{Deserialize, Serialize};

use crate::domain::DomainError;
use crate::domain::id::{ColumnId, IdGenerator, TableId};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SymbolRegistry {
    column_id_gen: IdGenerator,
    table_id_gen: IdGenerator,

    // Mapping Column: (TableId, Name) <-> ID
    col_name_to_id: HashMap<(TableId, String), ColumnId>,
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

    // --- COLUMN REGISTRY (SCOPED TO TABLE) ---

    pub fn register_column(&mut self, table_id: TableId, name: &str) -> ColumnId {
        let key = (table_id, name.to_lowercase());
        if let Some(&id) = self.col_name_to_id.get(&key) {
            return id;
        }

        let new_id = self.column_id_gen.next_column_id();
        self.col_name_to_id.insert(key, new_id);
        self.col_id_to_name.insert(new_id, name.to_string());
        new_id
    }

    /// Menghapus pendaftaran satu Kolom spesifik milik sebuah Table dari Registry
    pub fn unregister_column(
        &mut self,
        table_id: TableId,
        col_name: &str,
    ) -> Result<ColumnId, DomainError> {
        let key = (table_id, col_name.to_lowercase());

        // 1. Hapus dari col_name_to_id
        let col_id = self.col_name_to_id.remove(&key).ok_or_else(|| {
            DomainError::ColumnNotFound(format!("Kolom '{}' tidak ditemukan di Registry", col_name))
        })?;

        // 2. Hapus dari col_id_to_name
        self.col_id_to_name.remove(&col_id);

        Ok(col_id)
    }

    pub fn get_column_id(&self, table_id: TableId, name: &str) -> Option<ColumnId> {
        let key = (table_id, name.to_lowercase());
        self.col_name_to_id.get(&key).copied()
    }

    pub fn get_column_name(&self, id: ColumnId) -> Option<&str> {
        self.col_id_to_name.get(&id).map(|s| s.as_str())
    }

    pub fn rename_column(
        &mut self,
        table_id: TableId,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), DomainError> {
        let old_key = (table_id, old_name.to_lowercase());
        let new_key = (table_id, new_name.to_lowercase());

        let id = self.col_name_to_id.remove(&old_key).ok_or_else(|| {
            DomainError::ColumnNotFound(format!("Kolom '{}' tidak ditemukan di Registry", old_name))
        })?;

        if self.col_name_to_id.contains_key(&new_key) {
            self.col_name_to_id.insert(old_key, id);
            return Err(DomainError::ColumnAlreadyExists(format!(
                "Nama kolom baru '{}' sudah terpakai di Registry",
                new_name
            )));
        }

        self.col_name_to_id.insert(new_key, id);
        self.col_id_to_name.insert(id, new_name.to_string());
        Ok(())
    }

    // --- TABLE REGISTRY ---

    pub fn register_table(&mut self, name: &str) -> Result<TableId, DomainError> {
        let name_lower = name.to_lowercase();
        if self.table_name_to_id.contains_key(&name_lower) {
            return Err(DomainError::TableAlreadyExists(name.to_string()));
        }

        let new_id = self.table_id_gen.next_table_id();
        self.table_name_to_id.insert(name_lower, new_id);
        self.table_id_to_name.insert(new_id, name.to_string());
        Ok(new_id)
    }

    /// Menghapus pendaftaran Tabel dan SELURUH Kolom yang terikat padanya dari Registry
    pub fn unregister_table(&mut self, name: &str) -> Result<TableId, DomainError> {
        let name_lower = name.to_lowercase();

        // 1. Ambil & Hapus TableId berdasarkan nama
        let table_id = self.table_name_to_id.remove(&name_lower).ok_or_else(|| {
            DomainError::TableNotFound(format!("Table '{}' tidak ditemukan di Registry", name))
        })?;

        // 2. Hapus kebalikan mapping table_id_to_name
        self.table_id_to_name.remove(&table_id);

        // 3. Bersihkan seluruh kolom yang terkait dengan TableId ini
        // Retain hanya pasangan (TableId, String) yang TableId-nya BUKAN table_id yang dihapus
        let removed_col_ids: Vec<ColumnId> = self
            .col_name_to_id
            .iter()
            .filter(|((t_id, _), _)| *t_id == table_id)
            .map(|(_, &col_id)| col_id)
            .collect();

        // Hapus mapping col_name_to_id untuk tabel ini
        self.col_name_to_id.retain(|(t_id, _), _| *t_id != table_id);

        // Hapus mapping col_id_to_name untuk setiap ColumnId milik tabel ini
        for col_id in removed_col_ids {
            self.col_id_to_name.remove(&col_id);
        }

        Ok(table_id)
    }

    pub fn get_table_id(&self, name: &str) -> Option<TableId> {
        self.table_name_to_id.get(&name.to_lowercase()).copied()
    }

    pub fn get_table_name(&self, id: TableId) -> Option<&str> {
        self.table_id_to_name.get(&id).map(|s| s.as_str())
    }

    pub fn rename_table(&mut self, old_name: &str, new_name: &str) -> Result<(), DomainError> {
        let old_lower = old_name.to_lowercase();
        let new_lower = new_name.to_lowercase();

        let id = self.table_name_to_id.remove(&old_lower).ok_or_else(|| {
            DomainError::TableNotFound(format!("Table '{}' tidak ditemukan di Registry", old_name))
        })?;

        if self.table_name_to_id.contains_key(&new_lower) {
            self.table_name_to_id.insert(old_lower, id);
            return Err(DomainError::TableAlreadyExists(format!(
                "Nama Table baru '{}' sudah terpakai di Registry",
                new_name
            )));
        }

        self.table_name_to_id.insert(new_lower, id);
        self.table_id_to_name.insert(id, new_name.to_string());
        Ok(())
    }
}
