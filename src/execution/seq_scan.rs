//! Physical operator untuk Sequential Scan pada tabel berbasis halaman fisik.

use crate::execution::operator::PhysicalOperator;
use crate::{BufferPoolManager, DomainError, RID, Row, RowId, Schema, TableHeap, ValueType};

pub struct SeqScanOperator {
    table_heap: TableHeap,
    rids: Vec<RID>,
    cursor: usize,
    schema: Schema,
}

impl SeqScanOperator {
    /// Inisialisasi SeqScan dari TableHeap dan daftar RID yang telah dipindai
    #[inline]
    pub fn new(table_heap: TableHeap, rids: Vec<RID>, schema: Schema) -> Self {
        Self {
            table_heap,
            rids,
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
    fn next(&mut self, bpm: &mut BufferPoolManager) -> Result<Option<Row>, DomainError> {
        // Melakukan iterasi berdasarkan daftar RID fisik pada TableHeap[span_2](start_span)[span_2](end_span)
        while self.cursor < self.rids.len() {
            let rid = self.rids[self.cursor];
            self.cursor += 1;

            if let Some(tuple_bytes) = self.table_heap.get_tuple(bpm, rid)? {
                let values: Vec<ValueType> = bincode::deserialize(&tuple_bytes)
                    .map_err(|e| DomainError::storage(e.to_string()))?;

                let row_id = RowId::from(rid);
                return Ok(Some(Row::with_id(row_id, values)));
            }
        }

        Ok(None)
    }
}
