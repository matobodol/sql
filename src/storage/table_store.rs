use std::collections::HashMap;

use crate::ColumnId;
use crate::disk::{BufferPoolManager, TableHeap};
use crate::index::index_registry::IndexRegistry;

/// Konteks penyimpanan untuk setiap tabel fisik
#[derive(Debug)]
pub struct TableContext {
    pub table_heap: TableHeap,
    pub buffer_pool_manager: BufferPoolManager,
    pub index_registry: IndexRegistry,
    pub auto_increment_counters: HashMap<ColumnId, i64>,
}
