use crate::{DomainError, RID, Row};

pub trait TableReader {
    /// Mengambil satu baris spesifik berdasarkan RID fisik dari disk/buffer pool
    fn fetch_row_by_rid(&mut self, rid: RID) -> Result<Option<Row>, DomainError>;

    /// Membuat iterator untuk melakukan Sequential Scan secara lazy
    fn scan_rows(
        &mut self,
    ) -> Result<Box<dyn Iterator<Item = Result<Row, DomainError>> + '_>, DomainError>;
}
