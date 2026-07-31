use super::btree::BTreeIndex;
use super::traits::Index;
use crate::domain::domain_error::DomainError;
use crate::domain::id::{ColumnId, RowId};
use crate::domain::types::sql_value::SqlValue;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct IndexRegistry {
    /// Pemetaan dari ColumnId ke Indeks terkait.
    /// Satu kolom saat ini dipetakan ke 1 instance Index (BTreeIndex).
    indexes: HashMap<ColumnId, Box<dyn Index>>,
}

impl IndexRegistry {
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

    /// Menghapus indeks pada kolom tertentu (misal saat DROP COLUMN atau DROP INDEX).
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

    /// Menyisipkan nilai ke SEMUA indeks yang relevan saat ada baris baru (INSERT).
    /// `row_values`: Slice/Map berisi pasangan (ColumnId, SqlValue) milik baris baru.
    pub fn insert_entry(
        &mut self,
        row_id: RowId,
        row_values: &[(ColumnId, SqlValue)],
    ) -> Result<(), DomainError> {
        // Step 1: Validasi terlebih dahulu keunikan (UNIQUE check) di semua indeks sebelum melakukan penulisan.
        // Ini menjaga atomisitas agar tidak ada indeks yang terisi setengah jalan jika ada konflik keunikan.
        for (col_id, val) in row_values {
            if let Some(index) = self.indexes.get(col_id) {
                if index.is_unique() && !index.lookup(val).is_empty() {
                    return Err(DomainError::EvaluationError(format!(
                        "Pelanggaran keunikan indeks untuk ColumnId {:?}: Nilai '{:?}' sudah ada",
                        col_id, val
                    )));
                }
            }
        }

        // Step 2: Jika semua aman, lakukan mutasi insersi ke seluruh indeks
        for (col_id, val) in row_values {
            if let Some(index) = self.indexes.get_mut(col_id) {
                index.insert(val.clone(), row_id)?;
            }
        }

        Ok(())
    }

    /// Menghapus entri dari SEMUA indeks saat suatu baris dihapus (DELETE).
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
}
