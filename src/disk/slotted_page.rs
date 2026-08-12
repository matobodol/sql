use crate::{
    DomainError,
    disk::{PAGE_SIZE, PageId},
};

pub type SlotId = u16;

const HEADER_SIZE: usize = 8;
const SLOT_SIZE: usize = 4;
pub const INVALID_PAGE_ID: PageId = u32::MAX;

pub struct SlottedPage;

impl SlottedPage {
    pub fn init(page_data: &mut [u8; PAGE_SIZE]) {
        page_data[0..4].copy_from_slice(&INVALID_PAGE_ID.to_le_bytes());
        page_data[4..6].copy_from_slice(&0u16.to_le_bytes());
        let initial_free_space = PAGE_SIZE as u16;
        page_data[6..8].copy_from_slice(&initial_free_space.to_le_bytes());
    }

    pub fn next_page_id(page_data: &[u8; PAGE_SIZE]) -> PageId {
        PageId::from_le_bytes(page_data[0..4].try_into().unwrap())
    }

    pub fn set_next_page_id(page_data: &mut [u8; PAGE_SIZE], next_page_id: PageId) {
        page_data[0..4].copy_from_slice(&next_page_id.to_le_bytes());
    }

    pub fn num_slots(page_data: &[u8; PAGE_SIZE]) -> u16 {
        u16::from_le_bytes(page_data[4..6].try_into().unwrap())
    }

    fn set_num_slots(page_data: &mut [u8; PAGE_SIZE], count: u16) {
        page_data[4..6].copy_from_slice(&count.to_le_bytes());
    }

    pub fn free_space_pointer(page_data: &[u8; PAGE_SIZE]) -> u16 {
        u16::from_le_bytes(page_data[6..8].try_into().unwrap())
    }

    fn set_free_space_pointer(page_data: &mut [u8; PAGE_SIZE], ptr: u16) {
        page_data[6..8].copy_from_slice(&ptr.to_le_bytes());
    }

    pub fn free_space_remaining(page_data: &[u8; PAGE_SIZE]) -> usize {
        let num_slots = Self::num_slots(page_data) as usize;
        let free_ptr = Self::free_space_pointer(page_data) as usize;
        let header_end = HEADER_SIZE + (num_slots * SLOT_SIZE);

        if free_ptr < header_end {
            0
        } else {
            free_ptr - header_end
        }
    }

    pub fn insert_tuple(
        page_data: &mut [u8; PAGE_SIZE],
        tuple_bytes: &[u8],
    ) -> Result<SlotId, DomainError> {
        let tuple_len = tuple_bytes.len();
        let needed_space = tuple_len + SLOT_SIZE;

        if Self::free_space_remaining(page_data) < needed_space {
            return Err(DomainError::storage(
                "Kapasitas halaman tidak mencukupi untuk tuple",
            ));
        }

        let num_slots = Self::num_slots(page_data);
        let current_free_ptr = Self::free_space_pointer(page_data);

        let new_free_ptr = current_free_ptr - tuple_len as u16;
        let tuple_offset = new_free_ptr as usize;

        page_data[tuple_offset..tuple_offset + tuple_len].copy_from_slice(tuple_bytes);

        let slot_entry_offset = HEADER_SIZE + (num_slots as usize * SLOT_SIZE);
        page_data[slot_entry_offset..slot_entry_offset + 2]
            .copy_from_slice(&new_free_ptr.to_le_bytes());
        page_data[slot_entry_offset + 2..slot_entry_offset + 4]
            .copy_from_slice(&(tuple_len as u16).to_le_bytes());

        let slot_id = num_slots;
        Self::set_num_slots(page_data, num_slots + 1);
        Self::set_free_space_pointer(page_data, new_free_ptr);

        Ok(slot_id)
    }

    pub fn get_tuple<'a>(page_data: &'a [u8; PAGE_SIZE], slot_id: SlotId) -> Option<&'a [u8]> {
        let num_slots = Self::num_slots(page_data);
        if slot_id >= num_slots {
            return None;
        }

        let slot_entry_offset = HEADER_SIZE + (slot_id as usize * SLOT_SIZE);
        let tuple_offset = u16::from_le_bytes(
            page_data[slot_entry_offset..slot_entry_offset + 2]
                .try_into()
                .unwrap(),
        ) as usize;
        let tuple_size = u16::from_le_bytes(
            page_data[slot_entry_offset + 2..slot_entry_offset + 4]
                .try_into()
                .unwrap(),
        ) as usize;

        if tuple_size == 0 {
            return None;
        }

        Some(&page_data[tuple_offset..tuple_offset + tuple_size])
    }

    pub fn mark_delete(page_data: &mut [u8; PAGE_SIZE], slot_id: SlotId) -> bool {
        let num_slots = Self::num_slots(page_data);
        if slot_id >= num_slots {
            return false;
        }

        let slot_entry_offset = HEADER_SIZE + (slot_id as usize * SLOT_SIZE);
        page_data[slot_entry_offset + 2..slot_entry_offset + 4]
            .copy_from_slice(&0u16.to_le_bytes());
        true
    }
}
