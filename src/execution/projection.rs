//! Physical operator untuk mengeksekusi Proyeksi SQL (`SELECT expr1, expr2, ...`).

use super::operator::PhysicalOperator;
use crate::{
    domain::{DomainError, Row, Schema},
    eval_expr,
    expr::Expr,
};

/// Physical operator yang bertugas melakukan proyeksi daftar ekspresi (`SELECT`)
/// dan memetakan baris input ke bentuk skema keluaran baru.
pub struct ProjectionOperator {
    /// Physical operator anak yang menjadi sumber input stream data.
    input: Box<dyn PhysicalOperator>,
    /// Daftar ekspresi yang dievaluasi untuk membentuk tiap nilai kolom pada baris baru.
    exprs: Vec<Expr>,
    /// Skema keluaran hasil proyeksi.
    output_schema: Schema,
}

impl ProjectionOperator {
    /// Membuat instance `ProjectionOperator` baru.
    ///
    /// # Arguments
    /// * `input` - Operator anak yang memasok baris data.
    /// * `exprs` - Daftar ekspresi SQL yang akan dievaluasi per baris.
    /// * `output_schema` - Skema baru yang merepresentasikan struktur keluaran hasil proyeksi.
    pub fn new(input: Box<dyn PhysicalOperator>, exprs: Vec<Expr>, output_schema: Schema) -> Self {
        Self {
            input,
            exprs,
            output_schema,
        }
    }
}

impl PhysicalOperator for ProjectionOperator {
    /// Mengembalikan skema keluaran baru hasil proyeksi.
    fn schema(&self) -> &Schema {
        &self.output_schema
    }

    /// Mengambil baris data berikutnya dari input stream dan mengevaluasi seluruh ekspresi proyeksi.
    fn next(&mut self) -> Result<Option<Row>, DomainError> {
        if let Some(row) = self.input.next()? {
            let mut projected_values = Vec::with_capacity(self.exprs.len());

            for expr in &self.exprs {
                // Pasang referensi self.input.schema() langsung tanpa .clone()!
                let val = eval_expr(expr, self.input.schema(), &row)?;
                projected_values.push(val);
            }

            // 💡 Gunakan Row::with_id (bisa meneruskan row.id() asli dari input)
            Ok(Some(Row::with_id(row.id(), projected_values)))
        } else {
            Ok(None)
        }
    }
}
