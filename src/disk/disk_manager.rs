use crate::DomainError;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

pub const PAGE_SIZE: usize = 4096;
pub type PageId = u32;

#[derive(Debug)]
pub struct DiskManager {
    file: File,
    num_pages: PageId,
}

impl DiskManager {
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self, DomainError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(db_path)
            .map_err(|e| DomainError::storage(e.to_string()))?;

        let file_length = file
            .metadata()
            .map_err(|e| DomainError::storage(e.to_string()))?
            .len();
        let num_pages = (file_length / PAGE_SIZE as u64) as PageId;

        Ok(Self { file, num_pages })
    }

    pub fn allocate_page(&mut self) -> Result<PageId, DomainError> {
        let new_page_id = self.num_pages;
        self.num_pages += 1;
        Ok(new_page_id)
    }

    pub fn write_page(
        &mut self,
        page_id: PageId,
        page_data: &[u8; PAGE_SIZE],
    ) -> Result<(), DomainError> {
        let offset = page_id as u64 * PAGE_SIZE as u64;
        self.file
            .seek(SeekFrom::Start(offset))
            .map_err(|e| DomainError::storage(e.to_string()))?;
        self.file
            .write_all(page_data)
            .map_err(|e| DomainError::storage(e.to_string()))?;
        self.file
            .flush()
            .map_err(|e| DomainError::storage(e.to_string()))?;
        Ok(())
    }

    pub fn read_page(
        &mut self,
        page_id: PageId,
        page_data: &mut [u8; PAGE_SIZE],
    ) -> Result<(), DomainError> {
        let offset = page_id as u64 * PAGE_SIZE as u64;
        if page_id >= self.num_pages {
            return Err(DomainError::storage(format!("PageOutOfBounds: {page_id}")));
        }
        self.file
            .seek(SeekFrom::Start(offset))
            .map_err(|e| DomainError::storage(e.to_string()))?;
        self.file
            .read_exact(page_data)
            .map_err(|e| DomainError::storage(e.to_string()))?;
        Ok(())
    }

    pub fn num_pages(&self) -> PageId {
        self.num_pages
    }
}
