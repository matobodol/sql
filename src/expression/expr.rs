use super::binary_op::BinaryOp;
use crate::ValueType;
use serde::{Deserialize, Serialize};

/// Merepresentasikan pohon ekspresi (expression tree) untuk evaluasi data, kondisi, dan predikat SQL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Expr {
    /// Nilai literal konstan (misalnya angka, teks, boolean).
    Literal(ValueType),
    /// Digunakan saat tahap Parsing (AST Nama Kolom string).
    Column(String),
    /// Digunakan saat tahap Execution untuk akses kolom berbasis indeks offset O(1).
    ColumnIndex(usize),
    /// Operasi biner yang menggabungkan dua ekspresi dengan operator tertentu.
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    // --- Unary Operators ---
    /// Operator logika NOT.
    Not(Box<Expr>),
    /// Pengecekan apakah ekspresi bernilai NULL (`IS NULL`).
    IsNull(Box<Expr>),
    /// Pengecekan apakah ekspresi tidak bernilai NULL (`IS NOT NULL`).
    IsNotNull(Box<Expr>),
    // --- List Predicate ---
    /// Pengecekan nilai dalam daftar (`IN (...)`).
    InList { expr: Box<Expr>, list: Vec<Expr> },
}

impl Expr {
    /// Membuat ekspresi operasi biner baru.
    pub fn binary(left: Expr, op: BinaryOp, right: Expr) -> Self {
        Expr::Binary {
            left: Box::new(left),
            op,
            right: Box::new(right),
        }
    }

    /// Membuat ekspresi kolom berdasarkan namanya.
    pub fn col(name: String) -> Self {
        Expr::Column(name)
    }

    /// Membuat ekspresi kolom berdasarkan indeks offset.
    #[inline]
    pub fn col_idx(idx: usize) -> Self {
        Expr::ColumnIndex(idx)
    }

    /// Membuat ekspresi nilai literal.
    pub fn lit(val: impl Into<ValueType>) -> Self {
        Expr::Literal(val.into())
    }

    /// Membuat ekspresi negasi (NOT).
    pub fn not(expr: Expr) -> Self {
        Expr::Not(Box::new(expr))
    }

    /// Membuat ekspresi pemeriksaan `IS NULL`.
    pub fn is_null(expr: Expr) -> Self {
        Expr::IsNull(Box::new(expr))
    }

    /// Membuat ekspresi pemeriksaan `IS NOT NULL`.
    pub fn is_not_null(expr: Expr) -> Self {
        Expr::IsNotNull(Box::new(expr))
    }

    /// Membuat ekspresi predikat `IN`.
    pub fn in_list(expr: Expr, list: Vec<Expr>) -> Self {
        Expr::InList {
            expr: Box::new(expr),
            list,
        }
    }
}
