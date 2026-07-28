use super::operator::PhysicalOperator;
use crate::domain::{DomainError, Row, Schema, SqlValue, Table};
use crate::planner::Catalog;

pub struct CreateTableOperator {
    catalog: Catalog,
    table: Table,
    executed: bool,
}

impl CreateTableOperator {
    pub fn new(catalog: Catalog, table: Table) -> Self {
        Self {
            catalog,
            table,
            executed: false,
        }
    }
}

impl PhysicalOperator for CreateTableOperator {
    fn schema(&self) -> &Schema {
        self.table.schema()
    }

    fn next(&mut self) -> Result<Option<Row>, DomainError> {
        if self.executed {
            return Ok(None);
        }

        self.catalog.register_table(self.table.clone())?;
        self.executed = true;

        Ok(None)
    }
}

pub struct InsertOperator {
    table: Table,
    rows_to_insert: Vec<Row>,
    executed: bool,
}

impl InsertOperator {
    pub fn new(table: Table, rows_to_insert: Vec<Row>) -> Self {
        Self {
            table,
            rows_to_insert,
            executed: false,
        }
    }
}

impl PhysicalOperator for InsertOperator {
    fn schema(&self) -> &Schema {
        static OUTPUT_SCHEMA: std::sync::OnceLock<Schema> = std::sync::OnceLock::new();
        OUTPUT_SCHEMA.get_or_init(|| {
            Schema::new(vec![crate::domain::ColumnDef::new(
                "inserted_count",
                crate::domain::SqlType::Int,
                false,
            )])
        })
    }

    fn next(&mut self) -> Result<Option<Row>, DomainError> {
        if self.executed {
            return Ok(None);
        }

        let inserted_count = self.table.insert_many(self.rows_to_insert.clone())?;
        self.executed = true;

        Ok(Some(Row::new(vec![SqlValue::Int(inserted_count as i64)])))
    }
}
