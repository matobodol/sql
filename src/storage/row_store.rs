use crate::{Row, RowId, SqlValue};
use std::collections::HashSet;
use std::sync::Arc;

/// Penyimpanan fisik baris yang terisolasi dengan operasi mutasi berefisiensi tinggi.
#[derive(Debug, Clone, Default)]
pub struct RowStore {
    rows: Arc<Vec<Row>>,
    next_row_id: u64,
}

impl RowStore {
    /// Membuat instance `RowStore` kosong dengan ID awal 1.
    pub fn new() -> Self {
        Self {
            rows: Arc::new(Vec::new()),
            next_row_id: 1,
        }
    }

    /// Mengambil referensi slice baris tanpa alokasi memori.
    #[inline]
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// Menambahkan kolom baru ke seluruh baris yang ada secara in-place.
    pub fn add_column_to_rows(&mut self, target_idx: usize, default_value: SqlValue) {
        let vec = Arc::make_mut(&mut self.rows);
        for row in vec.iter_mut() {
            let _ = row.insert(target_idx, default_value.clone());
        }
    }

    /// Mengganti seluruh isi baris data secara langsung.
    pub fn replace_rows(&mut self, new_rows: Vec<Row>) {
        let vec = Arc::make_mut(&mut self.rows);
        *vec = new_rows;
    }

    /// Menyisipkan sekumpulan baris baru dan mengembalikan jumlah baris yang berhasil dimasukkan.
    pub fn insert_rows(&mut self, new_rows: Vec<Vec<SqlValue>>) -> usize {
        let count = new_rows.len();
        if count == 0 {
            return 0;
        }

        let vec = Arc::make_mut(&mut self.rows);
        // Pre-alokasi kapasitas vektor untuk mencegah re-alokasi berulang
        vec.reserve(count);

        for values in new_rows {
            let row_id = RowId::from(self.next_row_id);
            self.next_row_id += 1;
            vec.push(Row::with_id(row_id, values));
        }

        count
    }

    /// Menghapus baris berdasarkan daftar indeks dalam skenario tunggal $O(N)$ tanpa pergeseran berulang.
    pub fn delete_rows_by_indices(&mut self, indices: Vec<usize>) {
        if indices.is_empty() {
            return;
        }

        let vec = Arc::make_mut(&mut self.rows);
        // Konversi indeks yang dihapus ke HashSet agar pencarian status hapus bernilai O(1)
        let to_delete: HashSet<usize> = indices.into_iter().collect();
        let mut current_idx = 0;

        // Gunakan retain untuk menyaring baris dalam 1 kali pass linear O(N)
        vec.retain(|_| {
            let keep = !to_delete.contains(&current_idx);
            current_idx += 1;
            keep
        });
    }

    /// Memperbarui baris berdasarkan pasangan indeks dan data `Row` baru secara in-place.
    pub fn update_rows_by_indices(&mut self, updates: Vec<(usize, Row)>) {
        if updates.is_empty() {
            return;
        }

        let vec = Arc::make_mut(&mut self.rows);
        for (idx, new_row) in updates {
            if let Some(slot) = vec.get_mut(idx) {
                *slot = new_row;
            }
        }
    }

    /// Mengambil ID baris berikutnya yang siap digunakan.
    #[inline]
    pub fn next_row_id(&self) -> u64 {
        self.next_row_id
    }

    /// Mengambil salinan `Arc` dari seluruh vektor baris.
    #[inline]
    pub fn rows_arc(&self) -> Arc<Vec<Row>> {
        Arc::clone(&self.rows)
    }
}
