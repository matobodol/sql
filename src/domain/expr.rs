use super::sql_type::SqlValue;

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Eq,
    NotEq,
    Gt,
    Lt,
    GtEq,
    LtEq,
    And,
    Or,
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
}
