use super::{binary_op::BinaryOp, expr::Expr};
use crate::{DomainError, Row, Schema, SqlBool, SqlValue};

pub fn eval_expr(expr: &Expr, schema: &Schema, row: &Row) -> Result<SqlValue, DomainError> {
    match expr {
        Expr::Literal(val) => Ok(val.clone()),

        Expr::Column(col_id) => {
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

        // --- Unary Operations ---
        Expr::Not(inner) => {
            let val = eval_expr(inner, schema, row)?;
            let sql_bool = SqlBool::try_from(&val)?;
            Ok(SqlValue::from(!sql_bool))
        }

        Expr::IsNull(inner) => {
            let val = eval_expr(inner, schema, row)?;
            Ok(SqlValue::Bool(val.is_null()))
        }

        Expr::IsNotNull(inner) => {
            let val = eval_expr(inner, schema, row)?;
            Ok(SqlValue::Bool(!val.is_null()))
        }

        // --- List Operations (IN) dengan 3VL ---
        Expr::InList { expr, list } => {
            let target_val = eval_expr(expr, schema, row)?;

            // Aturan SQL 3VL: Jika nilai target NULL, hasilnya NULL (Unknown)
            if target_val.is_null() {
                return Ok(SqlValue::Null);
            }

            let mut has_unknown = false;

            for item_expr in list {
                let item_val = eval_expr(item_expr, schema, row)?;
                match target_val.eq(&item_val) {
                    SqlBool::True => return Ok(SqlValue::Bool(true)),
                    SqlBool::Unknown => has_unknown = true,
                    SqlBool::False => {}
                }
            }

            if has_unknown {
                Ok(SqlValue::Null)
            } else {
                Ok(SqlValue::Bool(false))
            }
        }

        // --- Binary Operations ---
        Expr::Binary { left, op, right } => {
            let left_val = eval_expr(left, schema, row)?;
            let right_val = eval_expr(right, schema, row)?;
            eval_binary_op(&left_val, *op, &right_val)
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

pub fn eval_where(expr: &Expr, schema: &Schema, row: &Row) -> Result<bool, DomainError> {
    let result = eval_expr(expr, schema, row)?;
    let sql_bool = SqlBool::try_from(&result)?;
    Ok(sql_bool.is_true())
}
