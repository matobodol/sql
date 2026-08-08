//! Physical operator untuk pemindaian berbasis indeks (*Index Scan*).

use crate::{DomainError, Row, RowId, Schema, TableStorage, execution::operator::PhysicalOperator};
use std::collections::HashSet;

pub struct IndexScanOperator {
    matching_rows: Vec<Row>,
    cursor: usize,
    schema: Schema,
}

impl IndexScanOperator {
    pub fn new(table: &TableStorage, schema: Schema, target_row_ids: Vec<RowId>) -> Self {
        let valid_ids: HashSet<RowId> = target_row_ids.into_iter().collect();

        let matching_rows: Vec<Row> = table
            .row_store()
            .rows()
            .iter()
            .filter(|row| valid_ids.contains(&row.id()))
            .cloned()
            .collect();

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
    fn next(&mut self) -> Result<Option<Row>, DomainError> {
        // Optimasi: Single-pass bounds check menggunakan `.get()`
        if let Some(row) = self.matching_rows.get(self.cursor) {
            self.cursor += 1;
            Ok(Some(row.clone()))
        } else {
            Ok(None)
        }
    }
}
