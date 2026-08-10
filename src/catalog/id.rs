use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
pub struct UserId(pub u32);

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
    column_counters: HashMap<TableId, u32>,
    #[serde(
        serialize_with = "serialize_atomic_u32",
        deserialize_with = "deserialize_atomic_u32"
    )]
    user_counter: AtomicU32,
    #[serde(
        serialize_with = "serialize_atomic_u32",
        deserialize_with = "deserialize_atomic_u32"
    )]
    db_counter: AtomicU32,
    start_value: u32,
}

impl IdGenerator {
    pub fn new(start_value: u32) -> Self {
        Self {
            table_counter: AtomicU32::new(start_value),
            column_counters: HashMap::new(),
            user_counter: AtomicU32::new(start_value),
            db_counter: AtomicU32::new(start_value),
            start_value,
        }
    }

    #[inline]
    pub fn next_table_id(&mut self) -> TableId {
        let val = self.table_counter.fetch_add(1, Ordering::Relaxed);
        TableId(u32::try_from(val).expect("Table ID overflow: Melebihi batas u32::MAX!"))
    }

    #[inline]
    pub fn next_column_id(&mut self, table_id: TableId) -> ColumnId {
        let counter = self
            .column_counters
            .entry(table_id)
            .or_insert(self.start_value);
        let current = *counter;
        *counter = current
            .checked_add(1)
            .expect("Column ID overflow: Melebihi batas u32::MAX!");
        ColumnId(current)
    }

    #[inline]
    pub fn next_user_id(&mut self) -> UserId {
        let val = self.user_counter.fetch_add(1, Ordering::Relaxed);
        UserId(u32::try_from(val).expect("User ID overflow: Melebihi batas u32::MAX!"))
    }

    #[inline]
    pub fn next_database_id(&mut self) -> DatabaseId {
        let val = self.db_counter.fetch_add(1, Ordering::Relaxed);
        DatabaseId(u32::try_from(val).expect("Database ID overflow: Melebihi batas u32::MAX!"))
    }

    #[inline]
    pub fn reset_table_if_empty(&mut self, is_empty: bool) {
        if is_empty {
            self.table_counter
                .store(self.start_value, Ordering::Relaxed);
        }
    }

    #[inline]
    pub fn reset_user_if_empty(&mut self, is_empty: bool) {
        if is_empty {
            self.user_counter.store(self.start_value, Ordering::Relaxed);
        }
    }

    #[inline]
    pub fn reset_database_if_empty(&mut self, is_empty: bool) {
        if is_empty {
            self.db_counter.store(self.start_value, Ordering::Relaxed);
        }
    }

    #[inline]
    pub fn remove_table_counter(&mut self, table_id: TableId) {
        self.column_counters.remove(&table_id);
    }

    #[inline]
    pub fn reset_column_counter_if_empty(&mut self, table_id: TableId, is_empty: bool) {
        if is_empty {
            self.column_counters.insert(table_id, self.start_value);
        }
    }
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
