use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::command::QueryResult;
use crate::execution::aggregate::AggregateFunc;
use crate::execution::sort::OrderByExpr;
use crate::expr::Expr;
use crate::planner::PhysicalPlanner;
use crate::{ColumnId, Database, DomainError, Schema};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectStmt {
    pub projection: Vec<Expr>,
    pub selection: Option<Expr>,
    pub group_by: Vec<ColumnId>,
    pub aggregates: Vec<AggregateFunc>,
    pub order_by: Vec<OrderByExpr>,
    pub limit: Option<usize>,
    pub offset: usize,
}

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
