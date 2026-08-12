use std::fmt::Debug;
use std::ops::Bound;

use crate::{DomainError, RowId, ValueType, index::BTreeIndex};

/// Trait abstrak untuk seluruh jenis pengindeksan di database engine.
/// Menyediakan interface zero-allocation untuk pencarian dan mutasi baris.
pub trait Index: Debug + Send + Sync {
    /// Membuat salinan tertutup (boxed clone) dari instance indeks.
    fn clone_box(&self) -> Box<dyn Index>;

    /// Memasukkan entri `(&SqlValue, RowId)` ke dalam indeks secara zero-copy.
    fn insert(&mut self, key: &ValueType, row_id: RowId) -> Result<(), DomainError>;

    /// Menghapus `RowId` tertentu yang terasosiasi dengan `key` tanpa mengkloning key.
    fn remove(&mut self, key: &ValueType, row_id: RowId) -> Result<(), DomainError>;

    /// Mencari seluruh `RowId` yang cocok persis (*exact match*) tanpa alokasi vektor baru.
    fn lookup(&self, key: &ValueType) -> &[RowId];

    /// Mencari seluruh `RowId` dalam batas rentang (*range query*) dinamis.
    fn range_lookup(&self, min: Bound<&ValueType>, max: Bound<&ValueType>) -> Vec<RowId>;

    /// Memeriksa apakah indeks dikonfigurasi sebagai UNIQUE index.
    fn is_unique(&self) -> bool;

    /// Mengosongkan seluruh isi indeks secara in-place tanpa realokasi instance trait object.
    fn clear(&mut self);
}

impl Clone for Box<dyn Index> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

// Di dalam traits.rs
use serde::{Deserialize, Serialize}; // Sesuaikan path jika perlu

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexImpl {
    BTree(BTreeIndex),
}

impl Index for IndexImpl {
    fn clone_box(&self) -> Box<dyn Index> {
        match self {
            IndexImpl::BTree(btree) => Box::new(btree.clone()),
        }
    }

    fn insert(&mut self, key: &ValueType, row_id: RowId) -> Result<(), DomainError> {
        match self {
            IndexImpl::BTree(btree) => btree.insert(key, row_id),
        }
    }

    fn remove(&mut self, key: &ValueType, row_id: RowId) -> Result<(), DomainError> {
        match self {
            IndexImpl::BTree(btree) => btree.remove(key, row_id),
        }
    }

    fn lookup(&self, key: &ValueType) -> &[RowId] {
        match self {
            IndexImpl::BTree(btree) => btree.lookup(key),
        }
    }

    fn range_lookup(&self, min: Bound<&ValueType>, max: Bound<&ValueType>) -> Vec<RowId> {
        match self {
            IndexImpl::BTree(btree) => btree.range_lookup(min, max),
        }
    }

    fn is_unique(&self) -> bool {
        match self {
            IndexImpl::BTree(btree) => btree.is_unique(),
        }
    }

    fn clear(&mut self) {
        match self {
            IndexImpl::BTree(btree) => btree.clear(),
        }
    }
}
