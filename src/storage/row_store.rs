use crate::{Row, SqlValue, id::RowId};
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub struct RowStore {
    rows: Arc<Vec<Row>>,
    next_row_id: u64,
}

impl RowStore {
    pub fn new() -> Self {
        Self {
            rows: Arc::new(Vec::new()),
            next_row_id: 1,
        }
    }

    #[inline]
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    pub fn add_column_to_rows(&mut self, target_idx: usize, default_value: SqlValue) {
        let vec = Arc::make_mut(&mut self.rows);
        for row in vec.iter_mut() {
            let _ = row.insert(target_idx, default_value.clone());
        }
    }

    pub fn replace_rows(&mut self, new_rows: Vec<Row>) {
        let vec = Arc::make_mut(&mut self.rows);
        *vec = new_rows;
    }

    pub fn insert_rows(&mut self, new_rows: Vec<Vec<SqlValue>>) -> usize {
        let count = new_rows.len();
        if count == 0 {
            return 0;
        }

        let vec = Arc::make_mut(&mut self.rows);
        vec.reserve(count);

        for values in new_rows {
            let row_id = RowId::from(self.next_row_id);
            self.next_row_id += 1;
            vec.push(Row::with_id(row_id, values));
        }

        count
    }

    /// Menghapus baris berdasarkan daftar indeks secara efisien (in-place via Arc::make_mut).
    pub fn delete_rows_by_indices(&mut self, mut indices: Vec<usize>) {
        if indices.is_empty() {
            return;
        }

        // Urutkan dari terbesar ke terkecil agar indeks di depannya tidak tergeser saat remove
        indices.sort_unstable_by(|a, b| b.cmp(a));
        indices.dedup();

        let vec = Arc::make_mut(&mut self.rows);
        for idx in indices {
            if idx < vec.len() {
                vec.remove(idx);
            }
        }
    }

    /// Memperbarui baris berdasarkan daftar pasangan (indeks, Row baru) secara in-place.
    pub fn update_rows_by_indices(&mut self, updates: Vec<(usize, Row)>) {
        if updates.is_empty() {
            return;
        }

        let vec = Arc::make_mut(&mut self.rows);
        for (idx, new_row) in updates {
            if idx < vec.len() {
                vec[idx] = new_row;
            }
        }
    }

    #[inline]
    pub fn next_row_id(&self) -> u64 {
        self.next_row_id
    }

    #[inline]
    pub fn rows_arc(&self) -> Arc<Vec<Row>> {
        Arc::clone(&self.rows)
    }
}
