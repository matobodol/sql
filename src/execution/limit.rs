//! Physical operator untuk mengeksekusi pembatasan jumlah baris data (`LIMIT`) dan `OFFSET`.

use super::operator::PhysicalOperator;
use crate::{DomainError, Row, Schema, disk::BufferPoolManager};

pub struct LimitOperator {
    input: Box<dyn PhysicalOperator>,
    limit: Option<usize>,
    offset: usize,
    skipped: usize,
    produced: usize,
}

impl LimitOperator {
    #[inline]
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
    #[inline]
    fn schema(&self) -> &Schema {
        self.input.schema()
    }

    #[inline]
    fn next(&mut self, bpm: &mut BufferPoolManager) -> Result<Option<Row>, DomainError> {
        if let Some(limit) = self.limit {
            if self.produced >= limit {
                return Ok(None);
            }
        }

        while self.skipped < self.offset {
            if self.input.next(bpm)?.is_some() {
                self.skipped += 1;
            } else {
                return Ok(None);
            }
        }

        if let Some(row) = self.input.next(bpm)? {
            self.produced += 1;
            Ok(Some(row))
        } else {
            Ok(None)
        }
    }
}
