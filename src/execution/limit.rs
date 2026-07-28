use super::operator::PhysicalOperator;
use crate::domain::{DomainError, Row, Schema};

pub struct LimitOperator {
    input: Box<dyn PhysicalOperator>,
    limit: Option<usize>,
    offset: usize,
    skipped: usize,
    produced: usize,
}

impl LimitOperator {
    pub fn new(input: Box<dyn PhysicalOperator>, limit: Option<usize>, offset: usize) -> Self {
        Self {
            input,
            limit,
            offset,
            skipped: 0,
            produced: 0,
        }
    }
}

impl PhysicalOperator for LimitOperator {
    fn schema(&self) -> &Schema {
        self.input.schema()
    }

    fn next(&mut self) -> Result<Option<Row>, DomainError> {
        // Jika limit sudah tercapai, langsung hentikan eksekusi
        if let Some(limit) = self.limit {
            if self.produced >= limit {
                return Ok(None);
            }
        }

        // Skip baris data sebanyak `offset`
        while self.skipped < self.offset {
            if self.input.next()?.is_none() {
                return Ok(None); // Data habis sebelum offset terpenuhi
            }
            self.skipped += 1;
        }

        // Ambil baris data berikutnya yang valid
        if let Some(row) = self.input.next()? {
            self.produced += 1;
            Ok(Some(row))
        } else {
            Ok(None)
        }
    }
}
