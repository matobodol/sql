use crate::catalog::CatalogStore;
use crate::command::QueryResult;
use crate::execution::PhysicalPlanner;
use crate::{DomainError, Schema, SelectStmt, TableId, TableStorage};

pub(crate) fn execute_select(
    catalog: &CatalogStore,
    table_storage: &TableStorage,
    table_id: TableId,
    stmt: SelectStmt,
) -> Result<QueryResult, DomainError> {
    // let table_id = catalog.get_table_id(table_name)?;

    let schema_cols = catalog.get_schema_columns(table_id)?;

    let schema = Schema::new(schema_cols.to_vec())?;
    // let table_storage = catalog.get_table_storage(table_name)?;

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
