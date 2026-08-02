//! Physical operator untuk mengeksekusi penyaringan baris data (`FILTER` / `WHERE`).

use crate::execution::operator::PhysicalOperator;
use crate::expr::evaluator::eval_where;
use crate::{DomainError, Row, Schema, expr::Expr};

/// Physical operator yang bertugas memfilter baris data berdasarkan predikat ekspresi SQL.
pub struct FilterOperator {
    /// Physical operator anak yang menjadi sumber input stream data.
    input: Box<dyn PhysicalOperator>,
    /// Ekspresi logika yang digunakan sebagai kondisi/predikat penyaringan (`WHERE`).
    predicate: Expr,
}

impl FilterOperator {
    /// Membuat instance `FilterOperator` baru.
    ///
    /// # Arguments
    /// * `input` - Operator anak yang memasok baris data.
    /// * `predicate` - Ekspresi logika yang dievaluasi untuk setiap baris data.
    pub fn new(input: Box<dyn PhysicalOperator>, predicate: Expr) -> Self {
        Self { input, predicate }
    }
}

impl PhysicalOperator for FilterOperator {
    /// Mengembalikan skema dari input stream, karena operator penyaring tidak mengubah struktur kolom.
    fn schema(&self) -> &Schema {
        self.input.schema()
    }

    /// Mengambil baris berikutnya dari input stream yang memenuhi kondisi `predicate`.
    ///
    /// Mengikuti aturan ANSI SQL 3-Valued Logic (3VL): Hanya baris dengan hasil evaluasi `True` murni
    /// yang diteruskan. Hasil `False` maupun `Unknown` (`NULL`) akan diabaikan.
    fn next(&mut self) -> Result<Option<Row>, DomainError> {
        // Fetch baris secara berurutan sampai menemukan baris yang lolos predikat
        while let Some(row) = self.input.next()? {
            // Evaluasi predikat menggunakan helper SSOT eval_where
            if eval_where(&self.predicate, self.input.schema(), &row)? {
                return Ok(Some(row));
            }
        }

        // Stream data telah habis
        Ok(None)
    }
}
