use serde::{Deserialize, Serialize};

use super::binary_op::BinaryOp;
use crate::{ColumnId, SqlValue};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Expr {
    Literal(SqlValue),
    Column(ColumnId),
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
    /// Helper untuk membangun ekspresi biner
    pub fn binary(left: Expr, op: BinaryOp, right: Expr) -> Self {
        Expr::Binary {
            left: Box::new(left),
            op,
            right: Box::new(right),
        }
    }

    /// Helper instan untuk ekspresi kolom
    pub fn col(id: ColumnId) -> Self {
        Expr::Column(id)
    }

    /// Helper instan untuk ekspresi literal
    pub fn lit(val: impl Into<SqlValue>) -> Self {
        Expr::Literal(val.into())
    }

    /// Helper instan untuk NOT
    pub fn not(expr: Expr) -> Self {
        Expr::Not(Box::new(expr))
    }

    /// Helper instan untuk IS NULL
    pub fn is_null(expr: Expr) -> Self {
        Expr::IsNull(Box::new(expr))
    }

    /// Helper instan untuk IS NOT NULL
    pub fn is_not_null(expr: Expr) -> Self {
        Expr::IsNotNull(Box::new(expr))
    }

    /// Helper instan untuk IN (...)
    pub fn in_list(expr: Expr, list: Vec<Expr>) -> Self {
        Expr::InList {
            expr: Box::new(expr),
            list,
        }
    }
}
