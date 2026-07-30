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
    IsNull(Box<Expr>),
    IsNotNull(Box<Expr>),
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
}
