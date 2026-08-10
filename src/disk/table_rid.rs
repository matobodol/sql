use crate::{PageId, SlotId};

/// Record Identifier (RID) merepresentasikan lokasi unik satu Tuple/Row di disk.
/// Terdiri dari lokasi Page ID dan Slot ID di dalam Slotted Page tersebut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RID {
    pub page_id: PageId,
    pub slot_id: SlotId,
}

impl RID {
    pub fn new(page_id: PageId, slot_id: SlotId) -> Self {
        Self { page_id, slot_id }
    }
}
