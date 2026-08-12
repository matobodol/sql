use crate::{
    DomainError,
    disk::{BufferPoolManager, PageId, RID, SlottedPage, slotted_page::INVALID_PAGE_ID},
};

#[derive(Debug, Clone, Copy)]
pub struct TableHeap {
    first_page_id: PageId,
}

impl TableHeap {
    pub fn new(bpm: &mut BufferPoolManager) -> Result<Self, DomainError> {
        let (first_page_id, page_data) = bpm.new_page()?;
        SlottedPage::init(page_data);
        bpm.unpin_page(first_page_id, true)?;

        Ok(Self { first_page_id })
    }

    pub fn insert_tuple(
        &self,
        bpm: &mut BufferPoolManager,
        tuple_bytes: &[u8],
    ) -> Result<RID, DomainError> {
        let mut current_page_id = self.first_page_id;

        loop {
            let (page_data, _) = bpm.fetch_page(current_page_id)?;

            match SlottedPage::insert_tuple(page_data, tuple_bytes) {
                Ok(slot_id) => {
                    bpm.unpin_page(current_page_id, true)?;
                    return Ok(RID::new(current_page_id, slot_id));
                }
                Err(_) => {
                    let next_page_id = SlottedPage::next_page_id(page_data);

                    if next_page_id != INVALID_PAGE_ID {
                        bpm.unpin_page(current_page_id, false)?;
                        current_page_id = next_page_id;
                    } else {
                        bpm.unpin_page(current_page_id, false)?;

                        let (new_page_id, new_page_data) = bpm.new_page()?;
                        SlottedPage::init(new_page_data);

                        let slot_id = SlottedPage::insert_tuple(new_page_data, tuple_bytes)?;
                        bpm.unpin_page(new_page_id, true)?;

                        let (prev_page_data, _) = bpm.fetch_page(current_page_id)?;
                        SlottedPage::set_next_page_id(prev_page_data, new_page_id);
                        bpm.unpin_page(current_page_id, true)?;

                        return Ok(RID::new(new_page_id, slot_id));
                    }
                }
            }
        }
    }

    pub fn delete_tuple(&self, bpm: &mut BufferPoolManager, rid: RID) -> Result<bool, DomainError> {
        let (page_data, _) = bpm.fetch_page(rid.page_id)?;
        let success = SlottedPage::mark_delete(page_data, rid.slot_id);
        bpm.unpin_page(rid.page_id, success)?;
        Ok(success)
    }

    pub fn from_first_page(first_page_id: PageId) -> Self {
        Self { first_page_id }
    }

    pub fn first_page_id(&self) -> PageId {
        self.first_page_id
    }

    pub fn scan_rids(&self, bpm: &mut BufferPoolManager) -> Result<Vec<RID>, DomainError> {
        let mut rids = Vec::new();
        let mut current_page_id = self.first_page_id;

        while current_page_id != INVALID_PAGE_ID {
            let (page_data, _) = bpm.fetch_page(current_page_id)?;
            let num_slots = SlottedPage::num_slots(page_data);

            for slot_id in 0..num_slots {
                if SlottedPage::get_tuple(page_data, slot_id).is_some() {
                    rids.push(RID::new(current_page_id, slot_id));
                }
            }

            let next_page_id = SlottedPage::next_page_id(page_data);
            bpm.unpin_page(current_page_id, false)?;
            current_page_id = next_page_id;
        }

        Ok(rids)
    }

    pub fn get_tuple(
        &self,
        bpm: &mut BufferPoolManager,
        rid: RID,
    ) -> Result<Option<Vec<u8>>, DomainError> {
        let (page_data, _) = bpm.fetch_page(rid.page_id)?;
        let tuple_bytes = SlottedPage::get_tuple(page_data, rid.slot_id).map(|b| b.to_vec());
        bpm.unpin_page(rid.page_id, false)?;

        Ok(tuple_bytes)
    }
}
