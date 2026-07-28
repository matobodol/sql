use super::operator::PhysicalOperator;
use crate::domain::{DomainError, Expr, Row, Schema, SqlValue};

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
        // Skema FilterOperator sama persis dengan skema input-nya
        self.input.schema()
    }

    fn next(&mut self) -> Result<Option<Row>, DomainError> {
        let schema = self.input.schema().clone();

        // Loop sampai menemukan Row yang lolos filter, atau sampai data habis
        while let Some(row) = self.input.next()? {
            // Evaluasi predikat terhadap Row saat ini
            let eval_result = self.predicate.eval(&schema, &row)?;

            // Dalam logika SQL 3VL, hanya nilai TRUE murni yang lolos filter WHERE
            if let SqlValue::Bool(true) = eval_result {
                return Ok(Some(row));
            }
        }

        // Data habis
        Ok(None)
    }
}
