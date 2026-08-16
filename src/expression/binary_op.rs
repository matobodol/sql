use serde::{Deserialize, Serialize};

/// Operator perbandingan, aritmatika, dan logika SQL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOp {
    // Arithmetic
    /// Penjumlahan (`+`)
    Add,
    /// Pengurangan (`-`)
    Sub,
    /// Perkalian (`*`)
    Mul,
    /// Pembagian (`/`)
    Div,

    // Comparison
    /// Sama dengan (`=`)
    Eq,
    /// Tidak sama dengan (`!=` atau `<>`)
    NotEq,
    /// Kurang dari (`<`)
    Lt,
    /// Kurang dari atau sama dengan (`<=`)
    LtEq,
    /// Lebih dari (`>`)
    Gt,
    /// Lebih dari atau sama dengan (`>=`)
    GtEq,

    // Logical
    /// Logika AND
    And,
    /// Logika OR
    Or,
    /// Pencocokan pola string (`LIKE`)
    Like,
}
