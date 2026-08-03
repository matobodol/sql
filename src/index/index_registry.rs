use super::btree::BTreeIndex;
use super::traits::Index;
use crate::domain::domain_error::DomainError;
use crate::domain::id::{ColumnId, RowId};
use crate::domain::types::sql_value::SqlValue;
use std::collections::HashMap;

/// Registry pengelola seluruh indeks yang terdaftar pada sebuah tabel.
#[derive(Debug, Clone, Default)]
pub struct IndexRegistry {
    /// Pemetaan dari `ColumnId` ke implementasi `Index`.
    indexes: HashMap<ColumnId, Box<dyn Index>>,
}

impl IndexRegistry {
    /// Inisialisasi registry indeks kosong.
    pub fn new() -> Self {
        Self {
            indexes: HashMap::new(),
        }
    }

    /// Membuat BTreeIndex baru untuk kolom tertentu.
    pub fn create_btree_index(
        &mut self,
        col_id: ColumnId,
        is_unique: bool,
    ) -> Result<(), DomainError> {
        if self.indexes.contains_key(&col_id) {
            return Err(DomainError::EvaluationError(format!(
                "Indeks untuk ColumnId {:?} sudah ada",
                col_id
            )));
        }

        let index = BTreeIndex::new(is_unique);
        self.indexes.insert(col_id, Box::new(index));
        Ok(())
    }

    /// Menghapus indeks pada kolom tertentu.
    pub fn drop_index(&mut self, col_id: ColumnId) -> Option<Box<dyn Index>> {
        self.indexes.remove(&col_id)
    }

    /// Memeriksa apakah suatu kolom memiliki indeks.
    pub fn has_index(&self, col_id: ColumnId) -> bool {
        self.indexes.contains_key(&col_id)
    }

    /// Mengambil referensi read-only ke indeks pada kolom tertentu.
    pub fn get_index(&self, col_id: ColumnId) -> Option<&dyn Index> {
        self.indexes.get(&col_id).map(|idx| idx.as_ref())
    }

    /// Mendaftarkan entri baru ke seluruh indeks yang relevan secara atomik (dengan fitur rollback otomatis).
    pub fn insert_entry(
        &mut self,
        row_id: RowId,
        entries: &[(ColumnId, SqlValue)],
    ) -> Result<(), DomainError> {
        let mut inserted_indexes = Vec::new();

        for (col_id, val) in entries {
            if let Some(index) = self.indexes.get_mut(col_id) {
                if let Err(err) = index.insert(val.clone(), row_id) {
                    for (rb_col_id, rb_val) in inserted_indexes {
                        if let Some(rb_index) = self.indexes.get_mut(&rb_col_id) {
                            let _ = rb_index.remove(&rb_val, row_id);
                        }
                    }
                    return Err(err);
                }
                inserted_indexes.push((*col_id, val.clone()));
            }
        }

        Ok(())
    }

    /// Menghapus entri baris dari seluruh indeks terdaftar pada tabel saat operasi DELETE atau UPDATE.
    pub fn remove_entry(
        &mut self,
        row_id: RowId,
        row_values: &[(ColumnId, SqlValue)],
    ) -> Result<(), DomainError> {
        for (col_id, val) in row_values {
            if let Some(index) = self.indexes.get_mut(col_id) {
                index.remove(val, row_id)?;
            }
        }
        Ok(())
    }

    /// Menghapus SELURUH indeks yang terdaftar beserta data entri di dalamnya.
    pub fn clear(&mut self) {
        self.indexes.clear();
    }

    /// Mengosongkan seluruh data entri di dalam indeks,
    /// namun tetap mempertahankan skema indeks yang sudah terdaftar.
    pub fn clear_entries(&mut self) {
        // Jika hanya ingin mengosongkan entri tanpa menghapus definisi indeks
        for index in self.indexes.values_mut() {
            // Karena BTreeIndex di-box sebagai trait object,
            // kita bisa meng-instansiasi ulang BTreeIndex bersih berdasarkan is_unique-nya
            *index = Box::new(BTreeIndex::new(index.is_unique()));
        }
    }
}
