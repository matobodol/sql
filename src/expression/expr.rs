use super::binary_op::BinaryOp;
use crate::ValueType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Expr {
    Literal(ValueType),
    /// Digunakan saat Parsing (AST Nama Kolom)
    Column(String),
    /// Digunakan saat Execution (Indeks Offset Kolom O(1))
    ColumnIndex(usize),
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    // --- Unary Operators ---
    Not(Box<Expr>),
    IsNull(Box<Expr>),
    IsNotNull(Box<Expr>),
    // --- List Predicate ---
    InList {
        expr: Box<Expr>,
        list: Vec<Expr>,
    },
}

impl Expr {
    pub fn binary(left: Expr, op: BinaryOp, right: Expr) -> Self {
        Expr::Binary {
            left: Box::new(left),
            op,
            right: Box::new(right),
        }
    }

    pub fn col(name: String) -> Self {
        Expr::Column(name)
    }

    #[inline]
    pub fn col_idx(idx: usize) -> Self {
        Expr::ColumnIndex(idx)
    }

    pub fn lit(val: impl Into<ValueType>) -> Self {
        Expr::Literal(val.into())
    }

    pub fn not(expr: Expr) -> Self {
        Expr::Not(Box::new(expr))
    }

    pub fn is_null(expr: Expr) -> Self {
        Expr::IsNull(Box::new(expr))
    }

    pub fn is_not_null(expr: Expr) -> Self {
        Expr::IsNotNull(Box::new(expr))
    }

    pub fn in_list(expr: Expr, list: Vec<Expr>) -> Self {
        Expr::InList {
            expr: Box::new(expr),
            list,
        }
    }
}
