//! Kontrak trait utama untuk Physical Execution Operators berbasis Volcano Iterator Model[span_4](start_span)[span_4](end_span).

use crate::{BufferPoolManager, DomainError, Row, Schema};

/// Trait utama untuk semua Physical Operator dalam pipeline eksekusi query engine[span_5](start_span)[span_5](end_span).
pub trait PhysicalOperator {
    /// Mengembalikan skema (`Schema`) dari baris data yang dihasilkan oleh operator ini[span_6](start_span)[span_6](end_span).
    fn schema(&self) -> &Schema;

    /// Mengambil baris data (`Row`) berikutnya dari input stream menggunakan BufferPoolManager.
    fn next(&mut self, bpm: &mut BufferPoolManager) -> Result<Option<Row>, DomainError>;
}
