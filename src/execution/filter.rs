//! Physical operator untuk penyaringan baris data (`FILTER` / `WHERE`) dengan ekspresi yang sudah di-bind.

use std::sync::Arc;

use crate::disk::BufferPoolManager;
use crate::execution::operator::PhysicalOperator;
use crate::expression::eval_expr;
use crate::{Bool3VL, DomainError, Expr, Row, Schema};

/// Physical operator yang bertugas memfilter baris data berdasarkan predikat ter-bind O(1).
pub struct FilterOperator {
    input: Box<dyn PhysicalOperator>,
    /// Predikat yang sudah di-bind indeks kolomnya (O(1) access during eval)
    bound_predicate: Expr,
}

impl FilterOperator {
    /// Membuat instance `FilterOperator` baru dan langsung melakukan pre-binding pada predikat.
    pub fn new(input: Box<dyn PhysicalOperator>, predicate: Expr) -> Result<Self, DomainError> {
        let schema = input.schema();
        let bound_predicate = bind_expr_columns(&predicate, schema)?;

        Ok(Self {
            input,
            bound_predicate,
        })
    }
}

impl PhysicalOperator for FilterOperator {
    #[inline]
    fn schema(&self) -> &Schema {
        self.input.schema()
    }

    fn next(&mut self, bpm: &mut BufferPoolManager) -> Result<Option<Row>, DomainError> {
        while let Some(row) = self.input.next(bpm)? {
            let res = eval_expr(&self.bound_predicate, &row)?;
            let sql_bool = Bool3VL::try_from(res.as_ref())?;

            if sql_bool.is_true() {
                return Ok(Some(row));
            }
        }

        Ok(None)
    }
}

/// Pre-binding parser: Mengonversi `Expr::Column(name)` ke `Expr::ColumnIndex(offset)` O(1)
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
            let bound_list = list
                .iter()
                .map(|item| bind_expr_columns(item, schema))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Expr::InList {
                expr: Box::new(bound_target),
                list: bound_list,
            })
        }
        // Varian literal atau yang sudah ter-bind
        other => Ok(other.clone()),
    }
}
