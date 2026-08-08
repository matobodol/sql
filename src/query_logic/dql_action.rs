use std::sync::Arc;

use crate::catalog::CatalogStore;
use crate::command::QueryResult;
use crate::execution::PhysicalPlanner;
use crate::{
    Column, ColumnId, DataType, Database, DomainError, Row, RowId, Schema, SelectStmt, ValueType,
};

pub(crate) fn execute_select(
    db: &Database,
    table_name: &str,
    stmt: SelectStmt,
) -> Result<QueryResult, DomainError> {
    let table_id = db
        .catalog()
        .get_table_id(table_name)
        .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

    let schema_cols = db
        .catalog()
        .get_schema_columns(table_id)
        .ok_or_else(|| DomainError::TableNotFound(Arc::from(table_name)))?;

    let schema = Schema::new(schema_cols.to_vec())?;
    let table_storage = db.get_table_storage(table_name)?;

    let mut plan = PhysicalPlanner::build_plan(table_storage, &schema, &stmt)?;
    let final_schema = plan.schema().clone();

    let mut result_rows = match stmt.limit {
        Some(limit) => Vec::with_capacity(limit),
        None => Vec::new(),
    };

    while let Some(row) = plan.next()? {
        result_rows.push(row);
    }

    Ok(QueryResult::Dql {
        schema: final_schema,
        rows: result_rows,
    })
}

pub(crate) fn execute_show_tables(catalog: &CatalogStore) -> Result<QueryResult, DomainError> {
    let col_def = Column::new(ColumnId(1), "table_name", DataType::Text);
    let schema = Schema::new(vec![col_def])?;

    let table_names = catalog.list_tables();
    let mut rows = Vec::with_capacity(table_names.len());

    for (idx, name) in table_names.into_iter().enumerate() {
        let row_id = RowId((idx + 1) as u64);
        let values = vec![ValueType::Text(Arc::from(name))];
        rows.push(Row::with_id(row_id, values));
    }

    Ok(QueryResult::Dql { schema, rows })
}
