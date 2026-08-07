//! Physical operator untuk Sequential Scan pada tabel.

use crate::execution::operator::PhysicalOperator;
use crate::{DomainError, Row, Schema};
use std::sync::Arc;

pub struct SeqScanOperator {
    rows: Arc<Vec<Row>>,
    cursor: usize,
    schema: Schema,
}

impl SeqScanOperator {
    /// Inisialisasi SeqScan langsung dari Arc slice baris data
    #[inline]
    pub fn new(rows: Arc<Vec<Row>>, schema: Schema) -> Self {
        Self {
            rows,
            cursor: 0,
            schema,
        }
    }
}

impl PhysicalOperator for SeqScanOperator {
    #[inline]
    fn schema(&self) -> &Schema {
        &self.schema
    }

    #[inline]
    fn next(&mut self) -> Result<Option<Row>, DomainError> {
        // Optimasi: Eliminasi double bounds-checking menggunakan `.get()`
        if let Some(row) = self.rows.get(self.cursor) {
            self.cursor += 1;
            Ok(Some(row.clone()))
        } else {
            Ok(None)
        }
    }
}
