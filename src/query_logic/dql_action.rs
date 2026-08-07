use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::command::QueryResult;
use crate::execution::aggregate::AggregateFunc;
use crate::execution::sort::OrderByExpr;
use crate::expr::Expr;
use crate::id::{ColumnId, RowId};
use crate::planner::PhysicalPlanner;
use crate::{Column, Database, DomainError, Row, Schema, SqlType, SqlValue};

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

    // Optimasi Alokasi: Gunakan `limit` jika ada untuk pre-alokasi kapasitas Vec
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

pub(crate) fn show_tables(database: &Database) -> Result<QueryResult, DomainError> {
    let col_def = Column::new(ColumnId(1), "table_name", SqlType::Text);
    let schema = Schema::new(vec![col_def])?;

    let table_names = database.catalog().list_tables();
    let mut rows = Vec::with_capacity(table_names.len());

    for (idx, name) in table_names.into_iter().enumerate() {
        let row_id = RowId((idx + 1) as u64);
        let values = vec![SqlValue::Text(Arc::from(name))];
        rows.push(Row::with_id(row_id, values));
    }

    Ok(QueryResult::Dql { schema, rows })
}
