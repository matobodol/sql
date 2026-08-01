use std::sync::Arc;

use crate::{DomainError, PhysicalOperator, Row, Schema};

pub struct SeqScanOperator {
    // Memegang Arc dari data tabel, sehinga cloning-nya O(1) dan zero-copy data mentah
    rows: Arc<Vec<Row>>,
    cursor: usize,
    schema: Schema,
}

impl SeqScanOperator {
    pub fn new(rows: Arc<Vec<Row>>, schema: Schema) -> Self {
        Self {
            rows,
            cursor: 0,
            schema,
        }
    }
}

impl PhysicalOperator for SeqScanOperator {
    fn schema(&self) -> &Schema {
        &self.schema
    }

    fn next(&mut self) -> Result<Option<Row>, DomainError> {
        if self.cursor < self.rows.len() {
            // Mengambil baris data. Jika Row ringan, di-clone langsung.
            let row = self.rows[self.cursor].clone();
            self.cursor += 1;
            Ok(Some(row))
        } else {
            Ok(None)
        }
    }
}
