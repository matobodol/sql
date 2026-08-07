use super::{binary_op::BinaryOp, expr::Expr};
use crate::{DomainError, Row, SqlBool, SqlValue};
use std::borrow::Cow;

/// Evaluasi ekspresi berbasis Zero-Copy (mengembalikan Cow<&SqlValue>) dan Instant O(1) Access
pub fn eval_expr<'a>(expr: &'a Expr, row: &'a Row) -> Result<Cow<'a, SqlValue>, DomainError> {
    match expr {
        // Zero-copy: Langsung borrow referensi Literal dari AST
        Expr::Literal(val) => Ok(Cow::Borrowed(val)),

        // Evaluasi O(1) berbasis Offset Index Kolom
        Expr::ColumnIndex(idx) => {
            let val = row.get_by_index(*idx).ok_or_else(|| {
                DomainError::eval_error(format!("Row index out of bounds: {idx}"))
            })?;
            Ok(Cow::Borrowed(val))
        }

        // Fallback jika belum ter-bind (pencarian string tidak direkomendasikan pada hot-path)
        Expr::Column(name) => Err(DomainError::eval_error(format!(
            "Ekspresi kolom '{name}' belum di-bind ke ColumnIndex O(1)"
        ))),

        // --- Unary Operations ---
        Expr::Not(inner) => {
            let val = eval_expr(inner, row)?;
            let sql_bool = SqlBool::try_from(val.as_ref())?;
            Ok(Cow::Owned(SqlValue::from(!sql_bool)))
        }

        Expr::IsNull(inner) => {
            let val = eval_expr(inner, row)?;
            Ok(Cow::Owned(SqlValue::Bool(val.is_null())))
        }

        Expr::IsNotNull(inner) => {
            let val = eval_expr(inner, row)?;
            Ok(Cow::Owned(SqlValue::Bool(!val.is_null())))
        }

        // --- List Operations (IN) dengan 3VL ---
        Expr::InList { expr, list } => {
            let target_val = eval_expr(expr, row)?;

            if target_val.is_null() {
                return Ok(Cow::Owned(SqlValue::Null));
            }

            let mut has_unknown = false;

            for item_expr in list {
                let item_val = eval_expr(item_expr, row)?;

                // Memastikan passing &SqlValue ke .eq()
                match target_val.as_ref().eq(item_val.as_ref()) {
                    SqlBool::True => return Ok(Cow::Owned(SqlValue::Bool(true))),
                    SqlBool::Unknown => has_unknown = true,
                    SqlBool::False => {}
                }
            }

            if has_unknown {
                Ok(Cow::Owned(SqlValue::Null))
            } else {
                Ok(Cow::Owned(SqlValue::Bool(false)))
            }
        }

        // --- Binary Operations ---
        Expr::Binary { left, op, right } => {
            let left_val = eval_expr(left, row)?;
            let right_val = eval_expr(right, row)?;
            let res = eval_binary_op(left_val.as_ref(), *op, right_val.as_ref())?;
            Ok(Cow::Owned(res))
        }
    }
}

fn eval_binary_op(
    left: &SqlValue,
    op: BinaryOp,
    right: &SqlValue,
) -> Result<SqlValue, DomainError> {
    match op {
        // --- Perbandingan (3VL Domain SSOT) ---
        BinaryOp::Eq => Ok(SqlValue::from(left.eq(right))),
        BinaryOp::NotEq => Ok(SqlValue::from(left.noteq(right))),
        BinaryOp::Gt => Ok(SqlValue::from(left.gt(right))),
        BinaryOp::Lt => Ok(SqlValue::from(left.lt(right))),
        BinaryOp::GtEq => Ok(SqlValue::from(left.gteq(right))),
        BinaryOp::LtEq => Ok(SqlValue::from(left.lteq(right))),
        BinaryOp::Like => Ok(SqlValue::from(left.like(right)?)),

        // --- Logika (3VL Domain SSOT) ---
        BinaryOp::And => Ok(SqlValue::from(left.and(right)?)),
        BinaryOp::Or => Ok(SqlValue::from(left.or(right)?)),

        // --- Aritmatika (Domain SSOT) ---
        BinaryOp::Add => left.add(right),
        BinaryOp::Sub => left.sub(right),
        BinaryOp::Mul => left.mul(right),
        BinaryOp::Div => left.div(right),
    }
}

/// Evaluasi klausul WHERE tanpa alokasi terduplikasi
#[inline]
pub fn eval_where(expr: &Expr, row: &Row) -> Result<bool, DomainError> {
    let result = eval_expr(expr, row)?;
    let sql_bool = SqlBool::try_from(result.as_ref())?;
    Ok(sql_bool.is_true())
}
