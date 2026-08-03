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

// -- disk

use storage::TableHeap;
use storage::buffer::BufferPoolManager;
use storage::table::rid::RID;

/// Implementasi [`RowIterator`] berbasis Disk Storage Engine (`TableHeap` & `BufferPoolManager`).
pub struct DiskRowIterator<'a> {
    bpm: &'a mut BufferPoolManager,
    table_heap: &'a TableHeap,
    rids: Vec<RID>,
    cursor: usize,
}

impl<'a> DiskRowIterator<'a> {
    /// Membuat instance `DiskRowIterator` baru dari `TableHeap`.
    pub fn new(
        bpm: &'a mut BufferPoolManager,
        table_heap: &'a TableHeap,
    ) -> Result<Self, DomainError> {
        let rids = table_heap.scan_rids(bpm).map_err(|e| {
            DomainError::ExecutionError(format!("Gagal scan RID dari storage: {e}"))
        })?;

        Ok(Self {
            bpm,
            table_heap,
            rids,
            cursor: 0,
        })
    }
}

impl<'a> RowIterator for DiskRowIterator<'a> {
    fn next_row(&mut self) -> Result<Option<Row>, DomainError> {
        while self.cursor < self.rids.len() {
            let rid = self.rids[self.cursor];
            self.cursor += 1;

            let tuple_bytes = self.table_heap.get_tuple(self.bpm, rid).map_err(|e| {
                DomainError::ExecutionError(format!("I/O Error saat membaca tuple: {e}"))
            })?;

            if let Some(bytes) = tuple_bytes {
                let row = Row::from_bytes(&bytes)?;
                return Ok(Some(row));
            }
        }

        Ok(None)
    }
}
