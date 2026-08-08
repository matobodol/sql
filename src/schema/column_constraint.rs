use serde::{Deserialize, Serialize};

use crate::{AutoIncrement, Expr, ValueType};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ColumnConstraint {
    /// Kolom tidak boleh berisi SqlValue::Null
    NotNull,
    /// Nilai dalam kolom ini tidak boleh ada yang kembar di dalam tabel
    Unique,
    /// Menandakan kolom sebagai Primary Key
    PrimaryKey,
    /// Nilai bawaan jika saat INSERT nilai tidak disediakan
    Default(ValueType),
    /// Pengecekan ekspresi tingkat kolom (misal: length(username) > 3)
    Check(Expr),
    /// Generator nilai otomatis (misal: ID 1, 2, 3, dst.)
    AutoIncrement(AutoIncrement),
}
