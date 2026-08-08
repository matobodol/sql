use super::btree::BTreeIndex;
use super::traits::Index;
use crate::{ColumnId, DomainError, RowId, ValueType};
use std::collections::HashMap;

/// Registry terpusat yang mengelola seluruh indeks B-Tree pada kolom-kolom tabel.
#[derive(Debug, Clone, Default)]
pub struct IndexRegistry {
    indexes: HashMap<ColumnId, Box<dyn Index>>,
}

impl IndexRegistry {
    pub fn new() -> Self {
        Self {
            indexes: HashMap::new(),
        }
    }

    /// Mengecek apakah registry tidak memiliki indeks sama sekali.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.indexes.is_empty()
    }

    /// Mengembalikan jumlah indeks aktif di dalam registry.
    #[inline]
    pub fn len(&self) -> usize {
        self.indexes.len()
    }

    pub fn create_btree_index(
        &mut self,
        col_id: ColumnId,
        is_unique: bool,
    ) -> Result<(), DomainError> {
        if self.indexes.contains_key(&col_id) {
            return Err(DomainError::eval_error(format!(
                "Indeks untuk ColumnId {:?} sudah ada",
                col_id
            )));
        }

        let index = BTreeIndex::new(is_unique);
        self.indexes.insert(col_id, Box::new(index));
        Ok(())
    }

    pub fn drop_index(&mut self, col_id: ColumnId) -> Option<Box<dyn Index>> {
        self.indexes.remove(&col_id)
    }

    #[inline]
    pub fn has_index(&self, col_id: ColumnId) -> bool {
        self.indexes.contains_key(&col_id)
    }

    #[inline]
    pub fn get_index(&self, col_id: ColumnId) -> Option<&dyn Index> {
        self.indexes.get(&col_id).map(|idx| idx.as_ref())
    }

    /// Mendaftarkan entri baru secara Zero-Copy menggunakan referensi borrowed `&SqlValue`.
    pub fn insert_entry_ref(
        &mut self,
        row_id: RowId,
        entries: &[(ColumnId, &ValueType)],
    ) -> Result<(), DomainError> {
        let mut inserted_cols = Vec::with_capacity(entries.len());

        for &(col_id, val) in entries {
            if let Some(index) = self.indexes.get_mut(&col_id) {
                if let Err(err) = index.insert(val, row_id) {
                    // Rollback otomatis jika terjadi kegagalan/pelanggaran constraint
                    for (rb_col_id, rb_val) in inserted_cols {
                        if let Some(rb_index) = self.indexes.get_mut(&rb_col_id) {
                            let _ = rb_index.remove(rb_val, row_id);
                        }
                    }
                    return Err(err);
                }
                inserted_cols.push((col_id, val));
            }
        }

        Ok(())
    }

    /// Menghapus entri baris dari seluruh indeks secara atomic & zero-allocation.
    pub fn remove_entry_ref(
        &mut self,
        row_id: RowId,
        entries: &[(ColumnId, &ValueType)],
    ) -> Result<(), DomainError> {
        for &(col_id, val) in entries {
            if let Some(index) = self.indexes.get_mut(&col_id) {
                index.remove(val, row_id)?;
            }
        }
        Ok(())
    }

    pub fn clear(&mut self) {
        self.indexes.clear();
    }

    /// Mengosongkan entri di seluruh indeks secara in-place tanpa merealokasi Box wrapper.
    pub fn clear_entries(&mut self) {
        for index in self.indexes.values_mut() {
            index.clear();
        }
    }
}
