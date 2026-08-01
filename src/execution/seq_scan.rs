use crate::domain::{DomainError, Row, Schema};
use crate::execution::iterator::RowIterator;
use crate::execution::operator::PhysicalOperator;

/// Sequential Scan Operator yang murni berbasis abstraksi `RowIterator`.
pub struct SeqScanOperator {
    iterator: Box<dyn RowIterator>,
    schema: Schema,
}

impl SeqScanOperator {
    pub fn new(iterator: Box<dyn RowIterator>, schema: Schema) -> Self {
        Self { iterator, schema }
    }
}

impl PhysicalOperator for SeqScanOperator {
    fn schema(&self) -> &Schema {
        &self.schema
    }

    fn next(&mut self) -> Result<Option<Row>, DomainError> {
        self.iterator.next_row()
    }
}
