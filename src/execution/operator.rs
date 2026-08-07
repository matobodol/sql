//! Kontrak trait utama untuk Physical Execution Operators berbasis Volcano Iterator Model.

use crate::{DomainError, Row, Schema};

/// Trait utama untuk semua Physical Operator dalam pipeline eksekusi query engine.
pub trait PhysicalOperator {
    /// Mengembalikan skema (`Schema`) dari baris data yang dihasilkan oleh operator ini.
    fn schema(&self) -> &Schema;

    /// Mengambil baris data (`Row`) berikutnya dari input stream.
    ///
    /// # Nilai Kembalian
    /// * `Ok(Some(Row))` - Baris data berikutnya berhasil diproses.
    /// * `Ok(None)` - Stream data telah mencapai End-of-Stream (EOS).
    /// * `Err(DomainError)` - Terjadi kesalahan evaluasi/IO.
    fn next(&mut self) -> Result<Option<Row>, DomainError>;
}
