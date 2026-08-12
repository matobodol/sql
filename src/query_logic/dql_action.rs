use crate::catalog::CatalogStore;
use crate::command::QueryResult;
use crate::execution::PhysicalPlanner;
use crate::{BufferPoolManager, DomainError, Schema, SelectStmt, TableHeap, TableId};

pub(crate) fn execute_select(
    catalog: &CatalogStore,
    table_heap: &TableHeap,
    bpm: &mut BufferPoolManager,
    table_id: TableId,
    stmt: SelectStmt,
) -> Result<QueryResult, DomainError> {
    let schema_cols = catalog.get_schema_columns(table_id)?;
    let schema = Schema::new(schema_cols.to_vec())?;

    let mut plan = PhysicalPlanner::build_plan(table_heap, bpm, &schema, &stmt)?;
    let final_schema = plan.schema().clone();

    let mut result_rows = match stmt.limit {
        Some(limit) => Vec::with_capacity(limit),
        None => Vec::new(),
    };

    // Sertakan bpm ke dalam method plan.next(bpm)
    while let Some(row) = plan.next(bpm)? {
        result_rows.push(row);
    }

    Ok(QueryResult::Dql {
        schema: final_schema,
        rows: result_rows,
    })
}
