use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct ColumnId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct TableId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct DatabaseId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct RowId(pub u64);

impl RowId {
    #[inline(always)]
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl From<u32> for ColumnId {
    #[inline(always)]
    fn from(id: u32) -> Self {
        ColumnId(id)
    }
}
impl From<u32> for TableId {
    #[inline(always)]
    fn from(id: u32) -> Self {
        TableId(id)
    }
}
impl From<u32> for DatabaseId {
    #[inline(always)]
    fn from(id: u32) -> Self {
        DatabaseId(id)
    }
}
impl From<u64> for RowId {
    #[inline(always)]
    fn from(id: u64) -> Self {
        RowId(id)
    }
}
impl From<RowId> for u64 {
    #[inline(always)]
    fn from(row_id: RowId) -> Self {
        row_id.0
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IdGenerator {
    #[serde(
        serialize_with = "serialize_atomic_u64",
        deserialize_with = "deserialize_atomic_u64"
    )]
    counter: AtomicU64,
}

impl IdGenerator {
    pub fn new(start_value: u32) -> Self {
        Self {
            counter: AtomicU64::new(start_value as u64),
        }
    }

    #[inline]
    fn next_u64(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::Relaxed)
    }

    #[inline]
    fn next_u32(&self) -> u32 {
        let val = self.next_u64();
        u32::try_from(val).expect("Metadata ID overflow: Melebihi batas u32::MAX!")
    }

    #[inline]
    pub(crate) fn next_table_id(&self) -> TableId {
        TableId(self.next_u32())
    }

    #[inline]
    pub(crate) fn next_column_id(&self) -> ColumnId {
        ColumnId(self.next_u32())
    }
}

impl Default for IdGenerator {
    fn default() -> Self {
        Self::new(1)
    }
}

fn serialize_atomic_u64<S>(val: &AtomicU64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_u64(val.load(Ordering::Relaxed))
}

fn deserialize_atomic_u64<'de, D>(deserializer: D) -> Result<AtomicU64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let val = u64::deserialize(deserializer)?;
    Ok(AtomicU64::new(val))
}
