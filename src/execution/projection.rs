use super::operator::PhysicalOperator;
use crate::{
    domain::{DomainError, Row, Schema},
    eval_expr,
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
        if let Some(row) = self.input.next()? {
            let mut projected_values = Vec::with_capacity(self.exprs.len());

            for expr in &self.exprs {
                // Gunakan referensi self.input.schema() langsung tanpa .clone()!
                let val = eval_expr(expr, self.input.schema(), &row)?;
                projected_values.push(val);
            }

            Ok(Some(Row::new(projected_values)))
        } else {
            Ok(None)
        }
    }
}
