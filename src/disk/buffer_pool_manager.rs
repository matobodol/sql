use crate::{
    DomainError,
    disk::{DiskManager, FrameId, LRUReplacer, PAGE_SIZE, PageId},
};
use std::collections::HashMap;

#[derive(Debug)]
pub struct BufferPoolManager {
    disk_manager: DiskManager,
    replacer: LRUReplacer,
    page_table: HashMap<PageId, FrameId>,
    frame_to_page: Vec<Option<PageId>>,
    pool: Vec<[u8; PAGE_SIZE]>,
    pin_counts: Vec<u32>,
    is_dirty: Vec<bool>,
    free_list: Vec<FrameId>,
}

impl BufferPoolManager {
    pub fn new(disk_manager: DiskManager, pool_size: usize) -> Self {
        let mut free_list = Vec::with_capacity(pool_size);
        for i in (0..pool_size).rev() {
            free_list.push(i);
        }

        Self {
            disk_manager,
            replacer: LRUReplacer::new(pool_size),
            page_table: HashMap::new(),
            frame_to_page: vec![None; pool_size],
            pool: vec![[0u8; PAGE_SIZE]; pool_size],
            pin_counts: vec![0; pool_size],
            is_dirty: vec![false; pool_size],
            free_list,
        }
    }

    pub fn fetch_page(
        &mut self,
        page_id: PageId,
    ) -> Result<(&mut [u8; PAGE_SIZE], FrameId), DomainError> {
        if let Some(&frame_id) = self.page_table.get(&page_id) {
            self.pin_counts[frame_id] += 1;
            self.replacer.pin(frame_id);
            return Ok((&mut self.pool[frame_id], frame_id));
        }

        let frame_id = self.find_available_frame()?;

        if let Some(old_page_id) = self.frame_to_page[frame_id] {
            if self.is_dirty[frame_id] {
                self.disk_manager
                    .write_page(old_page_id, &self.pool[frame_id])?;
                self.is_dirty[frame_id] = false;
            }
            self.page_table.remove(&old_page_id);
        }

        self.disk_manager
            .read_page(page_id, &mut self.pool[frame_id])?;
        self.page_table.insert(page_id, frame_id);
        self.frame_to_page[frame_id] = Some(page_id);
        self.pin_counts[frame_id] = 1;
        self.replacer.pin(frame_id);

        Ok((&mut self.pool[frame_id], frame_id))
    }

    pub fn new_page(&mut self) -> Result<(PageId, &mut [u8; PAGE_SIZE]), DomainError> {
        let frame_id = self.find_available_frame()?;
        let new_page_id = self.disk_manager.allocate_page()?;

        if let Some(old_page_id) = self.frame_to_page[frame_id] {
            if self.is_dirty[frame_id] {
                self.disk_manager
                    .write_page(old_page_id, &self.pool[frame_id])?;
                self.is_dirty[frame_id] = false;
            }
            self.page_table.remove(&old_page_id);
        }

        self.pool[frame_id] = [0u8; PAGE_SIZE];

        self.page_table.insert(new_page_id, frame_id);
        self.frame_to_page[frame_id] = Some(new_page_id);
        self.pin_counts[frame_id] = 1;
        self.is_dirty[frame_id] = true;
        self.replacer.pin(frame_id);

        Ok((new_page_id, &mut self.pool[frame_id]))
    }

    pub fn unpin_page(&mut self, page_id: PageId, is_dirty: bool) -> Result<(), DomainError> {
        let &frame_id = self
            .page_table
            .get(&page_id)
            .ok_or_else(|| DomainError::storage(format!("PageOutOfBounds: {page_id}")))?;

        if is_dirty {
            self.is_dirty[frame_id] = true;
        }

        if self.pin_counts[frame_id] > 0 {
            self.pin_counts[frame_id] -= 1;
            if self.pin_counts[frame_id] == 0 {
                self.replacer.unpin(frame_id);
            }
        }

        Ok(())
    }

    pub fn flush_page(&mut self, page_id: PageId) -> Result<(), DomainError> {
        if let Some(&frame_id) = self.page_table.get(&page_id) {
            if self.is_dirty[frame_id] {
                self.disk_manager
                    .write_page(page_id, &self.pool[frame_id])?;
                self.is_dirty[frame_id] = false;
            }
        }
        Ok(())
    }

    pub fn flush_all_pages(&mut self) -> Result<(), DomainError> {
        let page_ids: Vec<PageId> = self.page_table.keys().copied().collect();
        for page_id in page_ids {
            self.flush_page(page_id)?;
        }
        Ok(())
    }

    fn find_available_frame(&mut self) -> Result<FrameId, DomainError> {
        if let Some(frame_id) = self.free_list.pop() {
            Ok(frame_id)
        } else if let Some(frame_id) = self.replacer.victim() {
            Ok(frame_id)
        } else {
            Err(DomainError::storage(
                "Buffer pool penuh (invalid buffer size)",
            ))
        }
    }

    pub fn num_pages(&self) -> PageId {
        self.disk_manager.num_pages()
    }
}

impl Drop for BufferPoolManager {
    fn drop(&mut self) {
        let _ = self.flush_all_pages();
    }
}
