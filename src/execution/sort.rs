use super::operator::PhysicalOperator;
use crate::{
    domain::{DomainError, Row, Schema},
    eval_expr,
    expr::Expr,
};
use std::cmp::Ordering;
use std::vec::IntoIter;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

#[derive(Debug, Clone)]
pub struct OrderByExpr {
    pub expr: Expr,
    pub order: SortOrder,
}

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

    fn fetch_and_sort(&mut self) -> Result<(), DomainError> {
        let schema = self.input.schema().clone();
        let mut rows = Vec::new();

        while let Some(row) = self.input.next()? {
            rows.push(row);
        }

        let order_by = &self.order_by;
        let mut sort_error: Option<DomainError> = None;

        rows.sort_by(|a, b| {
            if sort_error.is_some() {
                return Ordering::Equal;
            }

            for spec in order_by {
                let val_a = match eval_expr(&spec.expr, &schema, a) {
                    Ok(v) => v,
                    Err(e) => {
                        sort_error = Some(e);
                        return Ordering::Equal;
                    }
                };

                let val_b = match eval_expr(&spec.expr, &schema, b) {
                    Ok(v) => v,
                    Err(e) => {
                        sort_error = Some(e);
                        return Ordering::Equal;
                    }
                };

                // Langsung gunakan .cmp() bawaan Ord manual SqlValue
                let ord = val_a.cmp(&val_b);
                if ord != Ordering::Equal {
                    return match spec.order {
                        SortOrder::Ascending => ord,
                        SortOrder::Descending => ord.reverse(),
                    };
                }
            }

            Ordering::Equal
        });

        if let Some(err) = sort_error {
            return Err(err);
        }

        self.sorted_rows = Some(rows.into_iter());
        Ok(())
    }
}

impl PhysicalOperator for SortOperator {
    fn schema(&self) -> &Schema {
        self.input.schema()
    }

    fn next(&mut self) -> Result<Option<Row>, DomainError> {
        if self.sorted_rows.is_none() {
            self.fetch_and_sort()?;
        }

        Ok(self.sorted_rows.as_mut().unwrap().next())
    }
}
