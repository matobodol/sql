use super::btree::BTreeIndex;
use crate::{
    ColumnId, DomainError, RowId, ValueType,
    index::{Index, traits::IndexImpl},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Registry terpusat yang mengelola seluruh indeks pada kolom-kolom tabel.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexRegistry {
    indexes: HashMap<ColumnId, IndexImpl>,
}

impl IndexRegistry {
    pub fn new() -> Self {
        Self {
            indexes: HashMap::new(),
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.indexes.is_empty()
    }

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

        let btree = BTreeIndex::new(is_unique);
        self.indexes.insert(col_id, IndexImpl::BTree(btree));
        Ok(())
    }

    pub fn drop_index(&mut self, col_id: ColumnId) -> Option<IndexImpl> {
        self.indexes.remove(&col_id)
    }

    #[inline]
    pub fn has_index(&self, col_id: ColumnId) -> bool {
        self.indexes.contains_key(&col_id)
    }

    #[inline]
    pub fn get_index(&self, col_id: ColumnId) -> Option<&IndexImpl> {
        self.indexes.get(&col_id)
    }

    pub fn insert_entry_ref(
        &mut self,
        row_id: RowId,
        entries: &[(ColumnId, &ValueType)],
    ) -> Result<(), DomainError> {
        let mut inserted_cols = Vec::with_capacity(entries.len());

        for &(col_id, val) in entries {
            if let Some(index) = self.indexes.get_mut(&col_id) {
                let res = match index {
                    IndexImpl::BTree(btree) => btree.insert(val, row_id),
                };

                if let Err(err) = res {
                    for (rb_col_id, rb_val) in inserted_cols {
                        if let Some(rb_index) = self.indexes.get_mut(&rb_col_id) {
                            match rb_index {
                                IndexImpl::BTree(btree) => {
                                    let _ = btree.remove(rb_val, row_id);
                                }
                            }
                        }
                    }
                    return Err(err);
                }
                inserted_cols.push((col_id, val));
            }
        }

        Ok(())
    }

    pub fn remove_entry_ref(
        &mut self,
        row_id: RowId,
        entries: &[(ColumnId, &ValueType)],
    ) -> Result<(), DomainError> {
        for &(col_id, val) in entries {
            if let Some(index) = self.indexes.get_mut(&col_id) {
                match index {
                    IndexImpl::BTree(btree) => btree.remove(val, row_id)?,
                }
            }
        }
        Ok(())
    }

    pub fn clear(&mut self) {
        self.indexes.clear();
    }

    pub fn clear_entries(&mut self) {
        for index in self.indexes.values_mut() {
            match index {
                IndexImpl::BTree(btree) => btree.clear(),
            }
        }
    }
}
