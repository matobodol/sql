//! Physical operator untuk melakukan pemindaian berbasis indeks (*Index Scan*).

use crate::catalog::table::Table;
use crate::domain::{DomainError, Row, RowId, Schema};
use crate::execution::operator::PhysicalOperator;
use std::collections::HashSet;

/// Physical operator yang mengeksekusi pencarian baris langsung menggunakan daftar `RowId` dari BTreeIndex.
pub struct IndexScanOperator {
    /// Referensi baris-baris pada tabel yang dipindai.
    matching_rows: Vec<Row>,
    /// Indeks kursor untuk melacak baris saat ini.
    cursor: usize,
    /// Skema dari tabel.
    schema: Schema,
}

impl IndexScanOperator {
    /// Membuat `IndexScanOperator` dengan menyaring baris berdasarkan kandidat `RowId` terindeks.
    pub fn new(table: &Table, target_row_ids: Vec<RowId>) -> Self {
        let valid_ids: HashSet<RowId> = target_row_ids.into_iter().collect();

        // Ambil hanya baris fisik yang ID-nya cocok dengan kueri indeks
        let matching_rows: Vec<Row> = table
            .rows()
            .iter()
            .filter(|row| valid_ids.contains(&row.id()))
            .cloned()
            .collect();

        Self {
            matching_rows,
            cursor: 0,
            schema: table.schema().clone(),
        }
    }
}

impl PhysicalOperator for IndexScanOperator {
    fn schema(&self) -> &Schema {
        &self.schema
    }

    fn next(&mut self) -> Result<Option<Row>, DomainError> {
        if self.cursor < self.matching_rows.len() {
            let row = self.matching_rows[self.cursor].clone();
            self.cursor += 1;
            Ok(Some(row))
        } else {
            Ok(None)
        }
    }
}
