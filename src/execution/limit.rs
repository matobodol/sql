//! Physical operator untuk mengeksekusi pembatasan jumlah baris data (`LIMIT`) dan pengabaian awal baris data (`OFFSET`).

use super::operator::PhysicalOperator;
use crate::domain::{DomainError, Row, Schema};

/// Physical operator yang bertugas membatasi jumlah baris keluaran (`LIMIT`)
/// serta mengabaikan sejumlah baris pertama dari input stream (`OFFSET`).
pub struct LimitOperator {
    /// Physical operator anak yang menjadi sumber input stream data.
    input: Box<dyn PhysicalOperator>,
    /// Batas maksimum jumlah baris yang ingin dihasilkan (`None` berarti tanpa batas/unlimited).
    limit: Option<usize>,
    /// Jumlah baris awal dari input stream yang harus dilewati sebelum menghasilkan data.
    offset: usize,
    /// Penghitung jumlah baris awal yang telah dilewati/di-skip sejauh ini.
    skipped: usize,
    /// Penghitung jumlah baris yang telah berhasil diproduksi/diteruskan sejauh ini.
    produced: usize,
}

impl LimitOperator {
    /// Membuat instance `LimitOperator` baru.
    ///
    /// # Arguments
    /// * `input` - Operator anak yang memasok baris data.
    /// * `limit` - Jumlah baris maksimum yang diteruskan (`Option<usize>`).
    /// * `offset` - Jumlah baris awal yang diabaikan (`usize`).
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
    /// Mengembalikan skema dari input stream, karena operator `LIMIT`/`OFFSET` tidak mengubah struktur kolom.
    fn schema(&self) -> &Schema {
        self.input.schema()
    }

    /// Mengambil baris data berikutnya setelah mematuhi kriteria `OFFSET` dan `LIMIT`.
    fn next(&mut self) -> Result<Option<Row>, DomainError> {
        // 1. Cek apakah batas limit yang ditentukan sudah terpenuhi
        if let Some(limit) = self.limit {
            if self.produced >= limit {
                return Ok(None);
            }
        }

        // 2. Melewati (skip) baris data sebanyak `offset` di awal eksekusi stream
        while self.skipped < self.offset {
            if self.input.next()?.is_some() {
                self.skipped += 1;
            } else {
                // Input stream habis sebelum jumlah offset terpenuhi
                return Ok(None);
            }
        }

        // 3. Ambil baris data berikutnya dan perbarui statistik jumlah baris yang telah diproduksi
        if let Some(row) = self.input.next()? {
            self.produced += 1;
            Ok(Some(row))
        } else {
            Ok(None)
        }
    }
}
