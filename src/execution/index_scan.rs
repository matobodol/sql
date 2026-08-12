//! Physical operator untuk pemindaian berbasis indeks (*Index Scan*).

use crate::{BufferPoolManager, DomainError, Row, Schema, execution::operator::PhysicalOperator};

pub struct IndexScanOperator {
    matching_rows: Vec<Row>,
    cursor: usize,
    schema: Schema,
}

impl IndexScanOperator {
    pub fn new(schema: Schema, matching_rows: Vec<Row>) -> Self {
        Self {
            matching_rows,
            cursor: 0,
            schema,
        }
    }
}

impl PhysicalOperator for IndexScanOperator {
    #[inline]
    fn schema(&self) -> &Schema {
        &self.schema
    }

    #[inline]
    fn next(&mut self, bpm: &mut BufferPoolManager) -> Result<Option<Row>, DomainError> {
        // Parameter bpm disertakan sesuai kontrak trait PhysicalOperator
        let _ = bpm;

        // Optimasi: Single-pass bounds check menggunakan `.get()`
        if let Some(row) = self.matching_rows.get(self.cursor) {
            self.cursor += 1;
            Ok(Some(row.clone()))
        } else {
            Ok(None)
        }
    }
}
