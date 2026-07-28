use super::operator::PhysicalOperator;
use crate::domain::{DomainError, Row, Schema};

pub struct SeqScanOperator {
    schema: Schema,
    rows: Vec<Row>,
    cursor: usize,
}

impl SeqScanOperator {
    pub fn new(schema: Schema, rows: Vec<Row>) -> Self {
        Self {
            schema,
            rows,
            cursor: 0,
        }
    }
}

impl PhysicalOperator for SeqScanOperator {
    fn schema(&self) -> &Schema {
        &self.schema
    }

    fn next(&mut self) -> Result<Option<Row>, DomainError> {
        if self.cursor < self.rows.len() {
            let row = self.rows[self.cursor].clone();
            self.cursor += 1;
            Ok(Some(row))
        } else {
            Ok(None)
        }
    }
}
