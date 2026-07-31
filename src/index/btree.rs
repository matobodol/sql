use super::traits::Index;
use crate::domain::domain_error::DomainError;
use crate::domain::id::RowId;
use crate::domain::types::sql_value::SqlValue;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
pub struct BTreeIndex {
    /// Mapping dari nilai kolom (SqlValue) ke sekumpulan RowId.
    /// Menggunakan BTreeSet agar daftar RowId tersimpan secara efisien dan unik.
    map: BTreeMap<SqlValue, BTreeSet<RowId>>,
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

    fn insert(&mut self, key: SqlValue, row_id: RowId) -> Result<(), DomainError> {
        if self.is_unique {
            if let Some(existing_rows) = self.map.get(&key) {
                if !existing_rows.is_empty() {
                    return Err(DomainError::EvaluationError(format!(
                        "Pelanggaran keunikan indeks: Nilai '{:?}' sudah ada",
                        key
                    )));
                }
            }
        }

        self.map
            .entry(key)
            .or_insert_with(BTreeSet::new)
            .insert(row_id);

        Ok(())
    }

    fn remove(&mut self, key: &SqlValue, row_id: RowId) -> Result<(), DomainError> {
        if let Some(rows) = self.map.get_mut(key) {
            rows.remove(&row_id);
            if rows.is_empty() {
                self.map.remove(key);
            }
        }
        Ok(())
    }

    fn lookup(&self, key: &SqlValue) -> Vec<RowId> {
        self.map
            .get(key)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default()
    }

    fn range_lookup(&self, min: Option<&SqlValue>, max: Option<&SqlValue>) -> Vec<RowId> {
        use std::ops::Bound::*;

        let range_start = match min {
            Some(val) => Included(val),
            None => Unbounded,
        };

        let range_end = match max {
            Some(val) => Included(val),
            None => Unbounded,
        };

        self.map
            .range((range_start, range_end))
            .flat_map(|(_, rows)| rows.iter().copied())
            .collect()
    }

    fn is_unique(&self) -> bool {
        self.is_unique
    }
}
