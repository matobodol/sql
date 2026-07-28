use crate::domain::{DomainError, Row, Schema};

/// Trait utama untuk semua Operator Eksekusi (Volcano Iterator Model).
/// Setiap operator bekerja secara 'lazy' (memproses row hanya ketika `next()` dipanggil).
pub trait PhysicalOperator {
    /// Mengembalikan skema dari data yang dihasilkan oleh operator ini
    fn schema(&self) -> &Schema;

    /// Mengambil baris data berikutnya.
    /// Returns:
    /// - `Ok(Some(Row))` jika ada data yang lolos/dihasilkan.
    /// - `Ok(None)` jika data sudah habis (End of Stream).
    /// - `Err(DomainError)` jika terjadi kesalahan eksekusi.
    fn next(&mut self) -> Result<Option<Row>, DomainError>;
}
