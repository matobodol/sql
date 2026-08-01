use crate::domain::{DomainError, Row};
use std::sync::Arc;

/// Trait abstraksi sumber data baris logis.
/// `sql` engine hanya peduli pada interface ini tanpa tahu detail fisiknya.
pub trait RowIterator {
    fn next_row(&mut self) -> Result<Option<Row>, DomainError>;
}

/// Implementasi `RowIterator` berbasis data di RAM (Arc<Vec<Row>>).
/// Dipakai untuk in-memory execution, testing, atau prototyping.
pub struct MemoryRowIterator {
    rows: Arc<Vec<Row>>,
    cursor: usize,
}

impl MemoryRowIterator {
    pub fn new(rows: Arc<Vec<Row>>) -> Self {
        Self { rows, cursor: 0 }
    }
}

impl RowIterator for MemoryRowIterator {
    fn next_row(&mut self) -> Result<Option<Row>, DomainError> {
        if self.cursor < self.rows.len() {
            let row = self.rows[self.cursor].clone();
            self.cursor += 1;
            Ok(Some(row))
        } else {
            Ok(None)
        }
    }
}
