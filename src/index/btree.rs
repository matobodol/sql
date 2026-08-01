use std::collections::{BTreeMap, BTreeSet};
use std::ops::Bound;

use super::traits::Index;
use crate::domain::domain_error::DomainError;
use crate::domain::id::RowId;
use crate::domain::types::sql_value::SqlValue;

/// Implementasi BTree Index berbasis `BTreeMap`.
#[derive(Debug, Clone)]
pub struct BTreeIndex {
    /// Pemetaan dari `SqlValue` ke kumpulan `RowId` unik (`BTreeSet`).
    map: BTreeMap<SqlValue, BTreeSet<RowId>>,
    /// Status apakah indeks mewajibkan nilai unik.
    is_unique: bool,
}

impl BTreeIndex {
    /// Membuat instance `BTreeIndex` baru.
    pub fn new(is_unique: bool) -> Self {
        Self {
            map: BTreeMap::new(),
            is_unique,
        }
    }
}

impl Index for BTreeIndex {
    fn clone_box(&self) -> Box<dyn Index> {
        Box::new(self.clone())
    }

    /// Memasukkan entri `(SqlValue, RowId)` secara atomik.
    ///
    /// Sesuai Standar ANSI SQL:
    /// Jika `key` bernilai `NULL`, batasan `UNIQUE` diabaikan karena `NULL != NULL`.
    fn insert(&mut self, key: SqlValue, row_id: RowId) -> Result<(), DomainError> {
        match self.map.entry(key.clone()) {
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                // Sesuai standar SQL: Hanya nilai NON-NULL yang diperiksa keunikannya.
                if self.is_unique && !key.is_null() {
                    return Err(DomainError::InvalidExpression(format!(
                        "Pelanggaran keunikan indeks BTree pada nilai '{:?}'",
                        key
                    )));
                }

                // Masukkan RowId ke BTreeSet
                entry.get_mut().insert(row_id);
            }
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(BTreeSet::from([row_id]));
            }
        }

        Ok(())
    }

    /// Menghapus spesifik `RowId` dari entri `key`.
    fn remove(&mut self, key: &SqlValue, row_id: RowId) -> Result<(), DomainError> {
        if let Some(rows) = self.map.get_mut(key) {
            rows.remove(&row_id);
            if rows.is_empty() {
                self.map.remove(key);
            }
        }
        Ok(())
    }

    /// Mencari `RowId` dengan pencocokan nilai tepat (*exact match*).
    fn lookup(&self, key: &SqlValue) -> Vec<RowId> {
        self.map
            .get(key)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Mencari `RowId` dalam batas rentang menggunakan `std::ops::Bound`.
    fn range_lookup(&self, min: Bound<&SqlValue>, max: Bound<&SqlValue>) -> Vec<RowId> {
        self.map
            .range((min, max))
            .flat_map(|(_, rows)| rows.iter().copied())
            .collect()
    }

    /// Memeriksa apakah indeks bersifat UNIQUE.
    fn is_unique(&self) -> bool {
        self.is_unique
    }
}
