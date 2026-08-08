use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};

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

#[derive(Debug, Serialize, Deserialize)]
pub struct IdGenerator {
    #[serde(
        serialize_with = "serialize_atomic_u32",
        deserialize_with = "deserialize_atomic_u32"
    )]
    table_counter: AtomicU32,
    #[serde(
        serialize_with = "serialize_atomic_u32",
        deserialize_with = "deserialize_atomic_u32"
    )]
    column_counter: AtomicU32,
}

impl IdGenerator {
    pub fn new(start_value: u32) -> Self {
        Self {
            table_counter: AtomicU32::new(start_value),
            column_counter: AtomicU32::new(start_value),
        }
    }

    #[inline]
    pub(crate) fn next_table_id(&self) -> TableId {
        let val = self.table_counter.fetch_add(1, Ordering::Relaxed);
        TableId(u32::try_from(val).expect("Table ID overflow: Melebihi batas u32::MAX!"))
    }

    // #[inline]
    // pub(crate) fn next_column_id(&self) -> ColumnId {
    //     let val = self.column_counter.fetch_add(1, Ordering::Relaxed);
    //     ColumnId(u32::try_from(val).expect("Column ID overflow: Melebihi batas u32::MAX!"))
    // }
}

impl Default for IdGenerator {
    fn default() -> Self {
        Self::new(1)
    }
}

fn serialize_atomic_u32<S>(val: &AtomicU32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_u32(val.load(Ordering::Relaxed))
}

fn deserialize_atomic_u32<'de, D>(deserializer: D) -> Result<AtomicU32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let val = u32::deserialize(deserializer)?;
    Ok(AtomicU32::new(val))
}
