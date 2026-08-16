//! Physical operator untuk mengeksekusi pengurutan baris data (`ORDER BY ASC/DESC`).

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::sync::Arc;
use std::vec::IntoIter;

use super::operator::PhysicalOperator;
use crate::{
    DomainError, Expr, Row, Schema, ValueType, disk::BufferPoolManager, expression::eval_expr,
};

/// Menentukan arah pengurutan data (`ASC` atau `DESC`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortOrder {
    /// Pengurutan secara menaik (Ascending)
    Ascending,
    /// Pengurutan secara menurun (Descending)
    Descending,
}

/// Merepresentasikan ekspresi pengurutan beserta arahnya dalam klausa ORDER BY.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderByExpr {
    /// Ekspresi atau kolom yang menjadi acuan pengurutan
    pub expr: Expr,
    /// Arah urutan (Ascending atau Descending)
    pub order: SortOrder,
}

/// Operator fisik untuk melakukan pengurutan baris data dari operator anak.
pub struct SortOperator {
    input: Box<dyn PhysicalOperator>,
    order_by: Vec<OrderByExpr>,
    sorted_rows: Option<IntoIter<Row>>,
}

impl SortOperator {
    pub fn new(input: Box<dyn PhysicalOperator>, order_by: Vec<OrderByExpr>) -> Self {
        Self {
            input,
            order_by,
            sorted_rows: None,
        }
    }

    fn fetch_and_sort(&mut self, bpm: &mut BufferPoolManager) -> Result<(), DomainError> {
        let schema = self.input.schema();

        let mut bound_specs = Vec::with_capacity(self.order_by.len());
        for spec in &self.order_by {
            let bound = bind_expr_columns(&spec.expr, schema)?;
            bound_specs.push((bound, spec.order.clone()));
        }

        let mut annotated_rows: Vec<(Vec<ValueType>, Row)> = Vec::new();

        while let Some(row) = self.input.next(bpm)? {
            let mut keys = Vec::with_capacity(bound_specs.len());
            for (expr, _) in &bound_specs {
                let val = eval_expr(expr, &row)?;
                keys.push(val.into_owned());
            }
            annotated_rows.push((keys, row));
        }

        annotated_rows.sort_by(|(keys_a, _), (keys_b, _)| {
            for (i, (_, order)) in bound_specs.iter().enumerate() {
                let ord = keys_a[i].cmp(&keys_b[i]);
                if ord != Ordering::Equal {
                    return match order {
                        SortOrder::Ascending => ord,
                        SortOrder::Descending => ord.reverse(),
                    };
                }
            }
            Ordering::Equal
        });

        let sorted_rows: Vec<Row> = annotated_rows.into_iter().map(|(_, row)| row).collect();
        self.sorted_rows = Some(sorted_rows.into_iter());
        Ok(())
    }
}

impl PhysicalOperator for SortOperator {
    #[inline]
    fn schema(&self) -> &Schema {
        self.input.schema()
    }

    #[inline]
    fn next(&mut self, bpm: &mut BufferPoolManager) -> Result<Option<Row>, DomainError> {
        if self.sorted_rows.is_none() {
            self.fetch_and_sort(bpm)?;
        }

        if let Some(iter) = &mut self.sorted_rows {
            Ok(iter.next())
        } else {
            Ok(None)
        }
    }
}

fn bind_expr_columns(expr: &Expr, schema: &Schema) -> Result<Expr, DomainError> {
    match expr {
        Expr::Column(name) => {
            let idx = schema
                .get_column_index_by_name(name)
                .ok_or_else(|| DomainError::ColumnNotFound(Arc::from(name.as_str())))?;
            Ok(Expr::ColumnIndex(idx))
        }
        other => Ok(other.clone()),
    }
}
