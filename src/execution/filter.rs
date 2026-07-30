use crate::{DomainError, Row, Schema, SqlValue, eval_expr, expr::Expr};

use super::operator::PhysicalOperator;

pub struct FilterOperator {
    input: Box<dyn PhysicalOperator>,
    predicate: Expr,
}

impl FilterOperator {
    pub fn new(input: Box<dyn PhysicalOperator>, predicate: Expr) -> Self {
        Self { input, predicate }
    }
}

impl PhysicalOperator for FilterOperator {
    fn schema(&self) -> &Schema {
        self.input.schema()
    }

    fn next(&mut self) -> Result<Option<Row>, DomainError> {
        // Loop sampai menemukan Row yang lolos filter, atau sampai data habis
        while let Some(row) = self.input.next()? {
            // Pasang referensi &Schema langsung tanpa .clone()!
            let eval_result = eval_expr(&self.predicate, self.input.schema(), &row)?;

            // Dalam logika SQL 3VL, hanya nilai TRUE murni yang lolos filter WHERE
            if let SqlValue::Bool(true) = eval_result {
                return Ok(Some(row));
            }
        }

        // Data habis
        Ok(None)
    }
}
