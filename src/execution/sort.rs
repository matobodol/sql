//! Physical operator untuk mengeksekusi pengurutan baris data (`ORDER BY ASC/DESC`).

use super::operator::PhysicalOperator;
use crate::{
    domain::{DomainError, Row, Schema},
    eval_expr,
    expr::Expr,
};
use std::cmp::Ordering;
use std::vec::IntoIter;

/// Arah pengurutan data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SortOrder {
    /// Urutan menaik (Ascending, dari kecil ke besar).
    Ascending,
    /// Urutan menurun (Descending, dari besar ke kecil).
    Descending,
}

/// Spesifikasi pengurutan untuk suatu ekspresi/kolom tertentu.
#[derive(Debug, Clone)]
pub struct OrderByExpr {
    /// Ekspresi SQL yang nilai hasilnya dijadikan basis pengurutan.
    pub expr: Expr,
    /// Arah urutan (`Ascending` atau `Descending`).
    pub order: SortOrder,
}

/// Physical operator yang bertugas mengumpulkan seluruh baris data dari input stream,
/// mengurutkannya berdasarkan kriteria multi-ekspresi, dan menyajikannya secara berurutan.
pub struct SortOperator {
    /// Physical operator anak yang menjadi sumber input stream data[span_2](start_span)[span_2](end_span).
    input: Box<dyn PhysicalOperator>,
    /// Daftar kriteria pengurutan (`ORDER BY`)[span_3](start_span)[span_3](end_span).
    order_by: Vec<OrderByExpr>,
    /// Iterator penampung baris data yang telah diurutkan[span_4](start_span)[span_4](end_span).
    sorted_rows: Option<IntoIter<Row>>,
}

impl SortOperator {
    /// Membuat instance `SortOperator` baru[span_5](start_span)[span_5](end_span).
    ///
    /// # Arguments
    /// * `input` - Operator anak yang memasok baris data[span_6](start_span)[span_6](end_span).
    /// * `order_by` - Vektor spesifikasi ekspresi pengurutan[span_7](start_span)[span_7](end_span).
    pub fn new(input: Box<dyn PhysicalOperator>, order_by: Vec<OrderByExpr>) -> Self {
        Self {
            input,
            order_by,
            sorted_rows: None,
        }
    }

    /// Mengambil seluruh baris data dari input stream (*pipeline breaker*)
    /// lalu mengurutkannya berdasarkan aturan `order_by`[span_8](start_span)[span_8](end_span).
    fn fetch_and_sort(&mut self) -> Result<(), DomainError> {
        let schema = self.input.schema().clone();
        let mut rows = Vec::new();

        while let Some(row) = self.input.next()? {
            rows.push(row);
        }

        let order_by = &self.order_by;
        let mut sort_error: Option<DomainError> = None;

        rows.sort_by(|a, b| {
            if sort_error.is_some() {
                return Ordering::Equal;
            }

            for spec in order_by {
                let val_a = match eval_expr(&spec.expr, &schema, a) {
                    Ok(v) => v,
                    Err(e) => {
                        sort_error = Some(e);
                        return Ordering::Equal;
                    }
                };

                let val_b = match eval_expr(&spec.expr, &schema, b) {
                    Ok(v) => v,
                    Err(e) => {
                        sort_error = Some(e);
                        return Ordering::Equal;
                    }
                };

                // Langsung gunakan .cmp() bawaan Ord manual SqlValue[span_9](start_span)[span_9](end_span)
                let ord = val_a.cmp(&val_b);
                if ord != Ordering::Equal {
                    return match spec.order {
                        SortOrder::Ascending => ord,
                        SortOrder::Descending => ord.reverse(),
                    };
                }
            }

            Ordering::Equal
        });

        if let Some(err) = sort_error {
            return Err(err);
        }

        self.sorted_rows = Some(rows.into_iter());
        Ok(())
    }
}

impl PhysicalOperator for SortOperator {
    /// Mengembalikan skema dari input stream, karena operator `SORT` tidak mengubah struktur kolom[span_10](start_span)[span_10](end_span).
    fn schema(&self) -> &Schema {
        self.input.schema()
    }

    /// Mengambil baris data berikutnya dari hasil pengurutan yang tersimpan dalam cache iterator[span_11](start_span)[span_11](end_span).
    fn next(&mut self) -> Result<Option<Row>, DomainError> {
        if self.sorted_rows.is_none() {
            self.fetch_and_sort()?;
        }

        Ok(self.sorted_rows.as_mut().unwrap().next())
    }
}
