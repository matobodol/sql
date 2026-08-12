//! Physical operator untuk mengeksekusi Proyeksi SQL (`SELECT expr1, expr2, ...`).

use std::sync::Arc;

use super::operator::PhysicalOperator;
use crate::{BufferPoolManager, DomainError, Expr, Row, Schema, expression::eval_expr};

pub struct ProjectionOperator {
    input: Box<dyn PhysicalOperator>,
    /// Pre-bound expressions untuk evaluasi O(1)
    bound_exprs: Vec<Expr>,
    output_schema: Schema,
}

impl ProjectionOperator {
    pub fn new(
        input: Box<dyn PhysicalOperator>,
        exprs: Vec<Expr>,
        output_schema: Schema,
    ) -> Result<Self, DomainError> {
        let schema = input.schema();
        let mut bound_exprs = Vec::with_capacity(exprs.len());
        for e in &exprs {
            bound_exprs.push(bind_expr_columns(e, schema)?);
        }

        Ok(Self {
            input,
            bound_exprs,
            output_schema,
        })
    }
}

impl PhysicalOperator for ProjectionOperator {
    #[inline]
    fn schema(&self) -> &Schema {
        &self.output_schema
    }

    #[inline]
    fn next(&mut self, bpm: &mut BufferPoolManager) -> Result<Option<Row>, DomainError> {
        if let Some(row) = self.input.next(bpm)? {
            let mut projected_values = Vec::with_capacity(self.bound_exprs.len());

            for expr in &self.bound_exprs {
                let val = eval_expr(expr, &row)?;
                projected_values.push(val.into_owned());
            }

            Ok(Some(Row::with_id(row.id(), projected_values)))
        } else {
            Ok(None)
        }
    }
}

/// Helper internal pre-binding kolom
fn bind_expr_columns(expr: &Expr, schema: &Schema) -> Result<Expr, DomainError> {
    match expr {
        Expr::Column(name) => {
            let idx = schema
                .get_column_index_by_name(name)
                .ok_or_else(|| DomainError::ColumnNotFound(Arc::from(name.as_str())))?;
            Ok(Expr::ColumnIndex(idx))
        }
        Expr::Binary { left, op, right } => {
            let bound_left = bind_expr_columns(left, schema)?;
            let bound_right = bind_expr_columns(right, schema)?;
            Ok(Expr::Binary {
                left: Box::new(bound_left),
                op: *op,
                right: Box::new(bound_right),
            })
        }
        Expr::Not(inner) => Ok(Expr::Not(Box::new(bind_expr_columns(inner, schema)?))),
        Expr::IsNull(inner) => Ok(Expr::IsNull(Box::new(bind_expr_columns(inner, schema)?))),
        Expr::IsNotNull(inner) => Ok(Expr::IsNotNull(Box::new(bind_expr_columns(inner, schema)?))),
        Expr::InList { expr, list } => {
            let bound_target = bind_expr_columns(expr, schema)?;
            let mut bound_list = Vec::with_capacity(list.len());
            for item in list {
                bound_list.push(bind_expr_columns(item, schema)?);
            }
            Ok(Expr::InList {
                expr: Box::new(bound_target),
                list: bound_list,
            })
        }
        other => Ok(other.clone()),
    }
}
