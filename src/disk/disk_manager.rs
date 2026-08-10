use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;
use thiserror::Error;

pub const PAGE_SIZE: usize = 4096;
pub type PageId = u32;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Gagal melakukan operasi I/O pada disk: {0}")]
    IoError(#[from] io::Error),

    #[error("Ukuran buffer ({0} byte) tidak sesuai dengan PAGE_SIZE ({PAGE_SIZE} byte)")]
    InvalidBufferSize(usize),

    #[error("Mencoba membaca Page ID {0} yang melebihi ukuran file database")]
    PageOutOfBounds(PageId),
}

pub struct DiskManager {
    file: File,
    num_pages: PageId,
}

impl DiskManager {
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self, StorageError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(db_path)?;

        let file_length = file.metadata()?.len();
        let num_pages = (file_length / PAGE_SIZE as u64) as PageId;

        Ok(Self { file, num_pages })
    }

    /// Alokasikan PageId baru tanpa menulis byte sampah mentah ke disk.
    pub fn allocate_page(&mut self) -> Result<PageId, StorageError> {
        let new_page_id = self.num_pages;
        self.num_pages += 1;
        Ok(new_page_id)
    }

    pub fn write_page(
        &mut self,
        page_id: PageId,
        page_data: &[u8; PAGE_SIZE],
    ) -> Result<(), StorageError> {
        let offset = page_id as u64 * PAGE_SIZE as u64;
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write_all(page_data)?;
        self.file.flush()?;
        Ok(())
    }

    pub fn read_page(
        &mut self,
        page_id: PageId,
        page_data: &mut [u8; PAGE_SIZE],
    ) -> Result<(), StorageError> {
        let offset = page_id as u64 * PAGE_SIZE as u64;
        if page_id >= self.num_pages {
            return Err(StorageError::PageOutOfBounds(page_id));
        }
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read_exact(page_data)?;
        Ok(())
    }

    pub fn num_pages(&self) -> PageId {
        self.num_pages
    }
}
