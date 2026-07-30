use crate::{DomainError, Row, Schema, SqlBool, SqlValue};

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
                let l = left.eval(schema, row)?;
                let r = right.eval(schema, row)?;

                let res: SqlBool = match op {
                    // --- Operator Perbandingan ---
                    BinaryOp::Eq => l.eq(&r),
                    BinaryOp::Gt => l.gt(&r),
                    BinaryOp::Lt => l.lt(&r),
                    BinaryOp::GtEq => l.gteq(&r),
                    BinaryOp::LtEq => l.lteq(&r),

                    // --- Operator Logika (3VL AND / OR / NOT) ---
                    BinaryOp::And => l.and(&r),
                    BinaryOp::Or => l.or(&r),
                    BinaryOp::NotEq => l.noteq(&r),
                };

                Ok(res.into())
            }
        }
    }
}
