use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

// --- STRUCT ID TERATUR ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ColumnId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TableId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DatabaseId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RowId(pub u64);

// Trait From untuk kemudahan instansiasi
impl From<u32> for ColumnId {
    fn from(id: u32) -> Self {
        ColumnId(id)
    }
}
impl From<u32> for TableId {
    fn from(id: u32) -> Self {
        TableId(id)
    }
}
impl From<u32> for DatabaseId {
    fn from(id: u32) -> Self {
        DatabaseId(id)
    }
}
impl From<u64> for RowId {
    fn from(id: u64) -> Self {
        RowId(id)
    }
}

// --- SINGLE ID GENERATOR (64-bit) ---

/// Engine generator tunggal berbasis 64-bit untuk seluruh kebutuhan ID database.
#[derive(Debug, Serialize, Deserialize)]
pub struct IdGenerator {
    #[serde(
        serialize_with = "serialize_atomic_u64",
        deserialize_with = "deserialize_atomic_u64"
    )]
    counter: AtomicU64,
}

impl IdGenerator {
    pub fn new(start_value: u64) -> Self {
        Self {
            counter: AtomicU64::new(start_value),
        }
    }

    /// Ambil nilai mentah 64-bit berikutnya
    pub fn next_u64(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::SeqCst)
    }

    /// Khusus RowId: Mengembalikan RowId 64-bit langsung
    pub fn next_row_id(&self) -> RowId {
        RowId(self.next_u64())
    }

    /// Untuk ID Metadata (u32): Ambil nilai u64 lalu cast aman ke u32.
    /// Jika melebihi u32::MAX, akan panic atau di-wrap (sesuai ekspektasi batas metadata).
    fn next_u32(&self) -> u32 {
        let val = self.next_u64();
        u32::try_from(val).expect("Metadata ID overflow: Melebihi batas u32::MAX!")
    }

    pub fn next_table_id(&self) -> TableId {
        TableId(self.next_u32())
    }

    pub fn next_column_id(&self) -> ColumnId {
        ColumnId(self.next_u32())
    }

    pub fn current_u64(&self) -> u64 {
        self.counter.load(Ordering::SeqCst)
    }

    pub fn current_row_id(&self) -> RowId {
        RowId(self.current_u64())
    }
}

impl Clone for IdGenerator {
    fn clone(&self) -> Self {
        Self {
            counter: AtomicU64::new(self.counter.load(Ordering::Relaxed)),
        }
    }
}

impl Default for IdGenerator {
    fn default() -> Self {
        Self::new(1)
    }
}

// Custom Serde Helper untuk AtomicU64
fn serialize_atomic_u64<S>(val: &AtomicU64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_u64(val.load(Ordering::SeqCst))
}

fn deserialize_atomic_u64<'de, D>(deserializer: D) -> Result<AtomicU64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let val = u64::deserialize(deserializer)?;
    Ok(AtomicU64::new(val))
}
