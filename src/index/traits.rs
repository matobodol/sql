use std::fmt::Debug;
use std::ops::Bound;

use crate::id::RowId;
use crate::{DomainError, SqlValue};

/// Trait abstrak untuk seluruh jenis pengindeksan di database engine.
/// Menyediakan interface zero-allocation untuk pencarian dan mutasi baris.
pub trait Index: Debug + Send + Sync {
    /// Membuat salinan tertutup (boxed clone) dari instance indeks.
    fn clone_box(&self) -> Box<dyn Index>;

    /// Memasukkan entri `(&SqlValue, RowId)` ke dalam indeks secara zero-copy.
    fn insert(&mut self, key: &SqlValue, row_id: RowId) -> Result<(), DomainError>;

    /// Menghapus `RowId` tertentu yang terasosiasi dengan `key` tanpa mengkloning key.
    fn remove(&mut self, key: &SqlValue, row_id: RowId) -> Result<(), DomainError>;

    /// Mencari seluruh `RowId` yang cocok persis (*exact match*) tanpa alokasi vektor baru.
    fn lookup(&self, key: &SqlValue) -> &[RowId];

    /// Mencari seluruh `RowId` dalam batas rentang (*range query*) dinamis.
    fn range_lookup(&self, min: Bound<&SqlValue>, max: Bound<&SqlValue>) -> Vec<RowId>;

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
