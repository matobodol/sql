use crate::domain::{BinaryOp, DomainError, Expr, SqlValue};
use sqlparser::ast::{BinaryOperator as SqlBinaryOperator, Expr as SqlExpr, Value};

/// Memetakan `sqlparser::ast::Expr` menjadi `crate::domain::Expr` milik kita
pub fn map_expr(sql_expr: &SqlExpr) -> Result<Expr, DomainError> {
    match sql_expr {
        // 1. Literal / Value Constant
        SqlExpr::Value(val) => match val {
            Value::Number(num_str, _) => {
                if let Ok(i) = num_str.parse::<i64>() {
                    Ok(Expr::Literal(SqlValue::Int(i)))
                } else if let Ok(f) = num_str.parse::<f64>() {
                    Ok(Expr::Literal(SqlValue::Float(f)))
                } else {
                    Err(DomainError::InvalidExpression(format!(
                        "Format angka tidak valid: {num_str}"
                    )))
                }
            }
            Value::SingleQuotedString(s) | Value::DoubleQuotedString(s) => {
                Ok(Expr::Literal(SqlValue::Text(s.clone())))
            }
            Value::Boolean(b) => Ok(Expr::Literal(SqlValue::Bool(*b))),
            Value::Null => Ok(Expr::Literal(SqlValue::Null)),
            _ => Err(DomainError::InvalidExpression(format!(
                "Tipe literal belum didukung: {val:?}"
            ))),
        },

        // 2. Identifier / Nama Kolom (misal: `age`, `users.name`)
        SqlExpr::Identifier(ident) => Ok(Expr::Column(ident.value.clone())),
        SqlExpr::CompoundIdentifier(idents) => {
            // Mengambil nama kolom saja jika ada prefix tabel (misal: `users.name` -> `name`)
            let col_name = idents.last().map(|id| id.value.clone()).unwrap_or_default();
            Ok(Expr::Column(col_name))
        }

        // 3. Operasi Biner (misal: `age > 20`, `a AND b`)
        SqlExpr::BinaryOp { left, op, right } => {
            let domain_left = map_expr(left)?;
            let domain_right = map_expr(right)?;

            let domain_op = match op {
                SqlBinaryOperator::Eq => BinaryOp::Eq,
                SqlBinaryOperator::NotEq => BinaryOp::NotEq,
                SqlBinaryOperator::Gt => BinaryOp::Gt,
                SqlBinaryOperator::Lt => BinaryOp::Lt,
                SqlBinaryOperator::GtEq => BinaryOp::GtEq,
                SqlBinaryOperator::LtEq => BinaryOp::LtEq,
                SqlBinaryOperator::And => BinaryOp::And,
                SqlBinaryOperator::Or => BinaryOp::Or,
                _ => {
                    return Err(DomainError::InvalidExpression(format!(
                        "Operator biner belum didukung: {op:?}"
                    )));
                }
            };

            Ok(Expr::binary(domain_left, domain_op, domain_right))
        }

        // 4. IS NULL / IS NOT NULL
        SqlExpr::IsNull(expr) => Ok(Expr::IsNull(Box::new(map_expr(expr)?))),
        SqlExpr::IsNotNull(expr) => Ok(Expr::IsNotNull(Box::new(map_expr(expr)?))),

        _ => Err(DomainError::InvalidExpression(format!(
            "Ekspresi SQL belum didukung: {sql_expr:?}"
        ))),
    }
}
