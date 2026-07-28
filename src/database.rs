use crate::domain::{DomainError, Row, Schema};
use crate::planner::{Catalog, QueryPlanner};

/// Public Facade untuk SQL Engine
#[derive(Clone, Default)]
pub struct Database {
    catalog: Catalog,
}

impl Database {
    pub fn new() -> Self {
        Self {
            catalog: Catalog::new(),
        }
    }

    /// Eksekusi satu instruksi SQL dan kembalikan Schema beserta baris hasilnya
    pub fn execute(&self, sql: &str) -> Result<(Schema, Vec<Row>), DomainError> {
        let planner = QueryPlanner::new(&self.catalog);
        let mut plan = planner.build_plan(sql)?;

        let schema = plan.schema().clone();
        let mut rows = Vec::new();

        while let Some(row) = plan.next()? {
            rows.push(row);
        }

        Ok((schema, rows))
    }
}
