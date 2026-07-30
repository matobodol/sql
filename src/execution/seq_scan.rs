use super::operator::PhysicalOperator;
use crate::domain::{DomainError, Row, Schema};
use std::vec::IntoIter;

pub struct SeqScanOperator {
    schema: Schema,
    rows: IntoIter<Row>,
}

impl SeqScanOperator {
    pub fn new(schema: Schema, rows: Vec<Row>) -> Self {
        Self {
            schema,
            rows: rows.into_iter(),
        }
    }
}

impl PhysicalOperator for SeqScanOperator {
    fn schema(&self) -> &Schema {
        &self.schema
    }

    fn next(&mut self) -> Result<Option<Row>, DomainError> {
        // .next() milik IntoIter langsung memindahkan (move) ownership Row tanpa clone!
        Ok(self.rows.next())
    }
}
