use crate::domain::domain_error::DomainError;
use crate::domain::id::RowId; // Menggunakan RowId dari domain::id kamu
use crate::domain::types::sql_value::SqlValue;
use std::fmt::Debug;

pub trait Index: Debug + Send + Sync {
    /// Helper untuk melakukan cloning pada Box<dyn Index>
    fn clone_box(&self) -> Box<dyn Index>;

    /// Menyisipkan nilai kolom & RowId ke dalam indeks.
    /// Jika `is_unique` bernilai true dan key sudah ada, mengembalikan `DomainError`.
    fn insert(&mut self, key: SqlValue, row_id: RowId) -> Result<(), DomainError>;

    /// Menghapus pasangan key dan RowId dari indeks.
    fn remove(&mut self, key: &SqlValue, row_id: RowId) -> Result<(), DomainError>;

    /// Pencarian presisi O(log N) untuk klausa WHERE col = val / Unique Check.
    fn lookup(&self, key: &SqlValue) -> Vec<RowId>;

    /// Pencarian jangkauan (Range Query) untuk WHERE col >= min AND col <= max.
    fn range_lookup(&self, min: Option<&SqlValue>, max: Option<&SqlValue>) -> Vec<RowId>;

    /// Apakah indeks ini menegakkan aturan keunikan (PRIMARY KEY / UNIQUE).
    fn is_unique(&self) -> bool;
}

// Implementasikan Clone khusus untuk Box<dyn Index>
impl Clone for Box<dyn Index> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
