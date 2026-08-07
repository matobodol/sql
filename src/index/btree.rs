use std::collections::{BTreeMap, btree_map::Entry};
use std::ops::Bound;

use crate::id::RowId;
use crate::{DomainError, SqlValue};

use super::traits::Index;

/// Implementasi BTree Index yang dioptimalkan memori & performanya.
#[derive(Debug, Clone)]
pub struct BTreeIndex {
    /// Pemetaan dari `SqlValue` ke kumpulan `RowId`
    map: BTreeMap<SqlValue, Vec<RowId>>,
    /// Status apakah indeks mewajibkan nilai unik.
    is_unique: bool,
}

impl BTreeIndex {
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

    /// Memasukkan entri secara zero-copy (hanya mengkloning key jika key belum ada di BTree)
    fn insert(&mut self, key: &SqlValue, row_id: RowId) -> Result<(), DomainError> {
        match self.map.entry(key.clone()) {
            Entry::Occupied(mut entry) => {
                // Sesuai standar SQL: Hanya nilai NON-NULL yang diperiksa keunikannya.
                if self.is_unique && !key.is_null() {
                    return Err(DomainError::invalid_expr(format!(
                        "Pelanggaran keunikan indeks BTree pada nilai '{:?}'",
                        key
                    )));
                }

                let rows = entry.get_mut();
                if !rows.contains(&row_id) {
                    rows.push(row_id);
                }
            }
            Entry::Vacant(entry) => {
                entry.insert(vec![row_id]);
            }
        }

        Ok(())
    }

    fn remove(&mut self, key: &SqlValue, row_id: RowId) -> Result<(), DomainError> {
        if let Entry::Occupied(mut entry) = self.map.entry(key.clone()) {
            let rows = entry.get_mut();
            rows.retain(|&id| id != row_id);
            if rows.is_empty() {
                entry.remove();
            }
        }
        Ok(())
    }

    fn lookup(&self, key: &SqlValue) -> Vec<RowId> {
        self.map.get(key).cloned().unwrap_or_default()
    }

    fn range_lookup(&self, min: Bound<&SqlValue>, max: Bound<&SqlValue>) -> Vec<RowId> {
        self.map
            .range((min, max))
            .flat_map(|(_, rows)| rows.iter().copied())
            .collect()
    }

    fn is_unique(&self) -> bool {
        self.is_unique
    }
}
