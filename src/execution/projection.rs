use super::operator::PhysicalOperator;
use crate::{
    domain::{DomainError, Row, Schema},
    expr::Expr,
};

pub struct ProjectionOperator {
    input: Box<dyn PhysicalOperator>,
    exprs: Vec<Expr>,
    output_schema: Schema,
}

impl ProjectionOperator {
    pub fn new(input: Box<dyn PhysicalOperator>, exprs: Vec<Expr>, output_schema: Schema) -> Self {
        Self {
            input,
            exprs,
            output_schema,
        }
    }
}

impl PhysicalOperator for ProjectionOperator {
    fn schema(&self) -> &Schema {
        &self.output_schema
    }

    fn next(&mut self) -> Result<Option<Row>, DomainError> {
        let input_schema = self.input.schema().clone();

        if let Some(row) = self.input.next()? {
            let mut projected_values = Vec::with_capacity(self.exprs.len());

            for expr in &self.exprs {
                let val = expr.eval(&input_schema, &row)?;
                projected_values.push(val);
            }

            Ok(Some(Row::new(projected_values)))
        } else {
            Ok(None)
        }
    }
}
