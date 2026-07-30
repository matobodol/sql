// #[derive(Debug, Clone, PartialEq, Eq)]
// pub enum BinaryOp {
//     Eq,    // =
//     NotEq, // != / <>
//     Gt,    // >
//     Lt,    // <
//     GtEq,  // >=
//     LtEq,  // <=
//     And,   // AND
//     Or,    // OR
// }

use serde::{Deserialize, Serialize};

/// Operator perbandingan dan logika SQL
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    // Comparison
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    // Logical
    And,
    Or,
}
