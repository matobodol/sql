use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ColumnId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TableId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DatabaseId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RowId(pub u64); // Gunakan u64 agar mampu menampung miliaran baris data

impl From<u64> for RowId {
    fn from(id: u64) -> Self {
        RowId(id)
    }
}
/// Generator ID sekuensial yang aman
#[derive(Debug, Serialize, Deserialize)]
pub struct IdGenerator {
    counter: AtomicU32,
}
// Implementasi Manual Clone
impl Clone for IdGenerator {
    fn clone(&self) -> Self {
        Self {
            counter: AtomicU32::new(self.counter.load(Ordering::Relaxed)),
        }
    }
}

impl IdGenerator {
    pub fn new(start_value: u32) -> Self {
        Self {
            counter: AtomicU32::new(start_value),
        }
    }

    pub fn next_id(&self) -> u32 {
        self.counter.fetch_add(1, Ordering::SeqCst)
    }
}

impl Default for IdGenerator {
    fn default() -> Self {
        Self::new(1)
    }
}
