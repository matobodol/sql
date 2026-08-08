use crate::{DomainError, Row, RowId};

/// Contract interface abstraksi antara SQL Execution Layer dan Storage Engine.
pub trait StorageEngine: Send + Sync {
    /// Menyimpan baris data baru dan mengembalikan RowId fisik.
    fn insert_row(&mut self, table_name: &str, row: &Row) -> Result<RowId, DomainError>;

    /// Membaca satu baris berdasarkan RowId.
    fn fetch_row(&self, table_name: &str, row_id: RowId) -> Result<Option<Row>, DomainError>;

    /// Menghapus baris berdasarkan RowId.
    fn delete_row(&mut self, table_name: &str, row_id: RowId) -> Result<bool, DomainError>;

    /// Membaca seluruh baris pada tabel (Full Table Scan).
    fn scan_rows(&mut self, table_name: &str) -> Result<Vec<(RowId, Row)>, DomainError>;
}
