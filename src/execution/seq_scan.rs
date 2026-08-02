//! Physical operator untuk melakukan pemindaian berurutan (*Sequential Scan*) pada sebuah tabel.

use crate::domain::{DomainError, Row, Schema};
use crate::execution::iterator::RowIterator;
use crate::execution::operator::PhysicalOperator;

/// Physical operator yang bertugas membungkus [`RowIterator`] untuk menyediakan
/// input stream data awal (Sequential Scan / Table Scan) ke dalam Volcano pipeline.
pub struct SeqScanOperator {
    /// Sumber data baris abstrak yang mengimplementasikan trait [`RowIterator`].
    iterator: Box<dyn RowIterator>,
    /// Skema dari tabel/relasi yang sedang dipindai.
    schema: Schema,
}

impl SeqScanOperator {
    /// Membuat instance `SeqScanOperator` baru.
    ///
    /// # Arguments
    /// * `iterator` - Abstraksi iterator sumber data baris (misal: `MemoryRowIterator` atau `DiskRowIterator`).
    /// * `schema` - Skema tabel yang dipindai.
    pub fn new(iterator: Box<dyn RowIterator>, schema: Schema) -> Self {
        Self { iterator, schema }
    }
}

impl PhysicalOperator for SeqScanOperator {
    /// Mengembalikan skema tabel yang dipindai.
    fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Mengambil baris data berikutnya dari storage layer iterator.
    fn next(&mut self) -> Result<Option<Row>, DomainError> {
        self.iterator.next_row()
    }
}
