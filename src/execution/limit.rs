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
        // 1. Cek apakah batas limit sudah terpenuhi
        if let Some(limit) = self.limit {
            if self.produced >= limit {
                return Ok(None);
            }
        }

        // 2. Skip baris data sebanyak `offset` (hanya berjalan saat offset belum terpenuhi)
        while self.skipped < self.offset {
            match self.input.next()? {
                Some(_) => self.skipped += 1,
                None => return Ok(None), // Data habis sebelum offset terpenuhi
            }
        }

        // 3. Ambil data baris berikutnya
        match self.input.next()? {
            Some(row) => {
                self.produced += 1;
                Ok(Some(row))
            }
            None => Ok(None),
        }
    }
}
