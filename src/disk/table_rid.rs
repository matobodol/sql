use crate::{PageId, RowId, SlotId};

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

// Asumsi struktur RID dan RowId Anda:
// pub struct RID { pub page_id: u32, pub slot_id: u16 }
// pub struct RowId(pub u64);

impl From<RID> for RowId {
    fn from(rid: RID) -> Self {
        // Geser page_id sebanyak 16 bit ke kiri, lalu gabungkan dengan slot_id
        let combined = ((rid.page_id as u64) << 16) | (rid.slot_id as u64);
        RowId(combined)
    }
}

impl From<RowId> for RID {
    fn from(row_id: RowId) -> Self {
        // Ambil kembali page_id dengan geser kanan 16 bit
        let page_id = (row_id.0 >> 16) as u32;
        // Ambil slot_id menggunakan bitwise AND mask 16-bit (0xFFFF)
        let slot_id = (row_id.0 & 0xFFFF) as u16;
        RID { page_id, slot_id }
    }
}
