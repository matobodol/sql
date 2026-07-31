use super::{binary_op::BinaryOp, expr::Expr};
use crate::{DomainError, Row, Schema, SqlValue};
use ordered_float::OrderedFloat;

/// Evaluator untuk mengevaluasi `Expr` terhadap skema dan baris data
pub fn eval_expr(expr: &Expr, schema: &Schema, row: &Row) -> Result<SqlValue, DomainError> {
    match expr {
        // 1. Literal
        Expr::Literal(val) => Ok(val.clone()),

        // 2. Column
        Expr::Column(col_id) => {
            // Pencarian O(N) sangat cepat berbasis perbandingan integer u32
            let idx = schema.index_of_id(*col_id).ok_or_else(|| {
                DomainError::EvaluationError(format!(
                    "ColumnId '{:?}' tidak ditemukan di skema saat evaluasi",
                    col_id
                ))
            })?;

            row.values()
                .get(idx)
                .cloned()
                .ok_or_else(|| DomainError::EvaluationError("Row index out of bounds".into()))
        }

        // 3. IS NULL / IS NOT NULL
        Expr::IsNull(inner) => {
            let val = eval_expr(inner, schema, row)?;
            Ok(SqlValue::Bool(matches!(val, SqlValue::Null)))
        }
        Expr::IsNotNull(inner) => {
            let val = eval_expr(inner, schema, row)?;
            Ok(SqlValue::Bool(!matches!(val, SqlValue::Null)))
        }

        // 4. Binary Operation
        Expr::Binary { left, op, right } => {
            let left_val = eval_expr(left, schema, row)?;
            let right_val = eval_expr(right, schema, row)?;
            eval_binary_op(&left_val, *op, &right_val)
        }
    }
}

/// Helper evaluasi operator binary
fn eval_binary_op(
    left: &SqlValue,
    op: BinaryOp,
    right: &SqlValue,
) -> Result<SqlValue, DomainError> {
    match op {
        // --- Operator Perbandingan & Logika (3VL -> SqlBool -> SqlValue) ---
        BinaryOp::Eq => Ok(left.eq(&right).into()),
        BinaryOp::NotEq => Ok(left.noteq(&right).into()),
        BinaryOp::Gt => Ok(left.gt(&right).into()),
        BinaryOp::Lt => Ok(left.lt(&right).into()),
        BinaryOp::GtEq => Ok(left.gteq(&right).into()),
        BinaryOp::LtEq => Ok(left.lteq(&right).into()),
        BinaryOp::And => Ok(left.and(&right).into()),
        BinaryOp::Or => Ok(left.or(&right).into()),

        // --- Operator Aritmatika ---
        BinaryOp::Add => eval_arithmetic(&left, &right, |a, b| a + b, |a, b| a + b),
        BinaryOp::Sub => eval_arithmetic(&left, &right, |a, b| a - b, |a, b| a - b),
        BinaryOp::Mul => eval_arithmetic(&left, &right, |a, b| a * b, |a, b| a * b),
        BinaryOp::Div => {
            if is_zero(&right) {
                return Err(DomainError::EvaluationError(
                    "Pembagian dengan nol (Division by zero)".into(),
                ));
            }
            eval_arithmetic(&left, &right, |a, b| a / b, |a, b| a / b)
        }
    }
}

/// Helper evaluasi aritmatika (Mengikuti aturan ANSI SQL: NULL op ANYTHING = NULL)
fn eval_arithmetic<FInt, FFloat>(
    left: &SqlValue,
    right: &SqlValue,
    op_int: FInt,
    op_float: FFloat,
) -> Result<SqlValue, DomainError>
where
    FInt: Fn(i64, i64) -> i64,
    FFloat: Fn(f64, f64) -> f64,
{
    match (left, right) {
        (SqlValue::Null, _) | (_, SqlValue::Null) => Ok(SqlValue::Null),
        (SqlValue::Int(a), SqlValue::Int(b)) => Ok(SqlValue::Int(op_int(*a, *b))),
        (SqlValue::Float(a), SqlValue::Float(b)) => Ok(SqlValue::Float(OrderedFloat(op_float(
            a.into_inner(),
            b.into_inner(),
        )))),
        (SqlValue::Int(a), SqlValue::Float(b)) => Ok(SqlValue::Float(OrderedFloat(op_float(
            *a as f64,
            b.into_inner(),
        )))),
        (SqlValue::Float(a), SqlValue::Int(b)) => Ok(SqlValue::Float(OrderedFloat(op_float(
            a.into_inner(),
            *b as f64,
        )))),
        _ => Err(DomainError::EvaluationError(
            "Tipe data tidak valid untuk operasi aritmatika".into(),
        )),
    }
}

fn is_zero(val: &SqlValue) -> bool {
    match val {
        SqlValue::Int(0) => true,
        SqlValue::Float(f) => f.into_inner() == 0.0,
        _ => false,
    }
}

/// Helper khusus untuk mengevaluasi klausa WHERE.
/// Mengembalikan `true` jika dan hanya jika ekspresi bernilai `SqlValue::Bool(true)`.
pub fn eval_where(expr: &Expr, schema: &Schema, row: &Row) -> Result<bool, DomainError> {
    let result = eval_expr(expr, schema, row)?;
    match result {
        SqlValue::Bool(b) => Ok(b),
        _ => Ok(false), // Null atau tipe non-bool dianggap false dalam klausa WHERE
    }
}
