use crate::{DomainError, Row, Schema};

use super::sql_type::SqlValue;

/// Operator perbadingan SQL
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Eq,    // =
    NotEq, // !=
    Gt,    // >
    Lt,    // <
    GtEq,  // >=
    LtEq,  // <=
    And,   // &
    Or,    // |
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(SqlValue),
    Column(String),
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

    /// Evaluasi ekspresi terhadap skema dan baris data saat ini.
    pub fn eval(&self, schema: &Schema, row: &Row) -> Result<SqlValue, DomainError> {
        match self {
            // 1. Literal
            Expr::Literal(val) => Ok(val.clone()),

            // 2. Column: Cari nilai menggunakan get_by_name yang sudah kita buat di Row
            Expr::Column(name) => row.get_by_name(schema, name).cloned(),

            // 3. IS NULL / IS NOT NULL
            Expr::IsNull(expr) => {
                let val = expr.eval(schema, row)?;
                Ok(SqlValue::Bool(matches!(val, SqlValue::Null)))
            }
            Expr::IsNotNull(expr) => {
                let val = expr.eval(schema, row)?;
                Ok(SqlValue::Bool(!matches!(val, SqlValue::Null)))
            }

            // 4. Binary Operation
            Expr::Binary { left, op, right } => {
                let left_val = left.eval(schema, row)?;
                let right_val = right.eval(schema, row)?;

                match op {
                    BinaryOp::Eq => Ok(SqlValue::Bool(left_val.eq(&right_val).is_true())),
                    BinaryOp::Gt => Ok(SqlValue::Bool(left_val.gt(&right_val).is_true())),
                    // Pengoperasian operator lain bisa dilanjutkan bertahap
                    _ => Err(DomainError::EvaluationError(
                        "Operator belum didukung".into(),
                    )),
                }
            }
        }
    }
}
