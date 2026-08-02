//! Abstraksi iterator baris data untuk decoupling antara physical storage engine dan execution engine.

use crate::domain::{DomainError, Row};
use std::sync::Arc;

/// Trait abstraksi sumber data baris logis.
///
/// Query execution engine hanya berinteraksi melalui interface ini tanpa perlu
/// mengetahui detail spesifik penyimpanan fisik (in-memory, disk storage, atau network).
pub trait RowIterator {
    /// Mengambil baris berikutnya dari iterator.
    ///
    /// Mengembalikan `Ok(Some(Row))` jika baris berikutnya tersedia,
    /// `Ok(None)` jika stream data telah habis, atau `Err(DomainError)` jika terjadi kesalahan I/O / Storage.
    fn next_row(&mut self) -> Result<Option<Row>, DomainError>;
}

/// Implementasi [`RowIterator`] berbasis data yang disimpan dalam memori (`Arc<Vec<Row>>`).
///
/// Cocok digunakan untuk in-memory execution, testing, prototyping, maupun penyimpan cache internal.
pub struct MemoryRowIterator {
    /// Vector baris data yang dibungkus `Arc` untuk berbagai kepemilikan yang efisien.
    rows: Arc<Vec<Row>>,
    /// Indeks kursor yang menunjuk ke posisi baris data saat ini.
    cursor: usize,
}

impl MemoryRowIterator {
    /// Membuat instance `MemoryRowIterator` baru dari koleksi baris data berbasis `Arc`.
    pub fn new(rows: Arc<Vec<Row>>) -> Self {
        Self { rows, cursor: 0 }
    }
}

impl RowIterator for MemoryRowIterator {
    /// Mengambil baris data berikutnya berdasarkan indeks kursor.
    fn next_row(&mut self) -> Result<Option<Row>, DomainError> {
        if let Some(row) = self.rows.get(self.cursor) {
            self.cursor += 1;
            Ok(Some(row.clone()))
        } else {
            Ok(None)
        }
    }
}
