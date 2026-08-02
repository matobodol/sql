//! Kontrak trait utama untuk Physical Execution Operators berbasis Volcano Iterator Model.

use crate::domain::{DomainError, Row, Schema};

/// Trait utama untuk semua Physical Operator dalam pipeline eksekusi query engine.
///
/// Mengimplementasikan **Volcano Iterator Model** (juga dikenal sebagai Push-based / Pull-based Stream Iterator)[span_2](start_span)[span_2](end_span).
/// Setiap operator bekerja secara *lazy* (hanya memproses dan mengambil baris data ketika method [`next`](PhysicalOperator::next) dipanggil)[span_3](start_span)[span_3](end_span).
pub trait PhysicalOperator {
    /// Mengembalikan skema (`Schema`) dari baris data yang dihasilkan oleh operator ini[span_4](start_span)[span_4](end_span).
    ///
    /// Skema ini digunakan oleh operator tingkat di atasnya untuk memetakan nama/ID kolom ke indeks array nilai.
    fn schema(&self) -> &Schema;

    /// Mengambil baris data (`Row`) berikutnya dari input stream[span_5](start_span)[span_5](end_span).
    ///
    /// # Nilai Kembalian
    /// * `Ok(Some(Row))` - Jika baris data berikutnya berhasil diproses dan siap diteruskan[span_6](start_span)[span_6](end_span).
    /// * `Ok(None)` - Jika stream data telah mencapai batas akhir (*End of Stream*)[span_7](start_span)[span_7](end_span).
    /// * `Err(DomainError)` - Jika terjadi kesalahan selama proses eksekusi/evaluasi[span_8](start_span)[span_8](end_span).
    fn next(&mut self) -> Result<Option<Row>, DomainError>;
}
