use crate::catalog::{Metadata, QueryResult};
use crate::disk::{BufferPoolManager, TableHeap};
use crate::execution::{PhysicalPlanner, SelectStatement};
use crate::{AggregateFunc, ColumnId, DomainError, Expr, OrderByExpr, Schema, TableId};

#[derive(Debug, Clone, PartialEq)]
pub struct Statement {
    pub projection: Vec<Expr>,
    pub selection: Option<Expr>,
    pub group_by: Vec<String>,
    pub aggregates: Vec<Aggregate>,
    pub order_by: Vec<OrderByExpr>,
    pub limit: Option<usize>,
    pub offset: usize,
}

/// Varian fungsi agregasi untuk input publik (berbasis nama kolom String).
#[derive(Debug, Clone, PartialEq)]
pub enum Aggregate {
    Count(Option<String>),
    Sum(String),
    Avg(String),
    Min(String),
    Max(String),
}

/// Fungsi helper untuk mem-bind UnboundAggregateFunc menjadi AggregateFunc internal.
pub fn bind_aggregate(
    agg: &Aggregate,
    get_col_id: &impl Fn(&str) -> Result<ColumnId, DomainError>,
) -> Result<AggregateFunc, DomainError> {
    match agg {
        Aggregate::Count(None) => Ok(AggregateFunc::Count(None)),
        Aggregate::Count(Some(name)) => {
            let id = get_col_id(name)?;
            Ok(AggregateFunc::Count(Some(id)))
        }
        Aggregate::Sum(name) => {
            let id = get_col_id(name)?;
            Ok(AggregateFunc::Sum(id))
        }
        Aggregate::Avg(name) => {
            let id = get_col_id(name)?;
            Ok(AggregateFunc::Avg(id))
        }
        Aggregate::Min(name) => {
            let id = get_col_id(name)?;
            Ok(AggregateFunc::Min(id))
        }
        Aggregate::Max(name) => {
            let id = get_col_id(name)?;
            Ok(AggregateFunc::Max(id))
        }
    }
}

pub(crate) fn execute_select(
    meta: &Metadata,
    table_heap: &TableHeap,
    bpm: &mut BufferPoolManager,
    table_id: TableId,
    stmt: Statement,
) -> Result<QueryResult, DomainError> {
    // -------------------- BUILD STATEMENT ------------------------
    // Closure standar untuk menerjemahkan nama kolom string menjadi ColumnId via katalog
    let get_col_id = |name: &str| meta.get_column_id(table_id, name);

    // 1. Bind group_by dari Vec<String> ke Vec<ColumnId>
    let mut group_by = Vec::with_capacity(stmt.group_by.len());
    for name in &stmt.group_by {
        group_by.push(get_col_id(name)?);
    }

    // 2. Bind aggregates menggunakan fungsi helper bind_aggregate
    let mut aggregates = Vec::with_capacity(stmt.aggregates.len());
    for agg in &stmt.aggregates {
        aggregates.push(bind_aggregate(agg, &get_col_id)?);
    }

    // 3. Susun Statement internal yang siap diproses oleh Physical Planner
    let statement = SelectStatement {
        projection: stmt.projection,
        selection: stmt.selection,
        group_by,
        aggregates,
        order_by: stmt.order_by,
        limit: stmt.limit,
        offset: stmt.offset,
    };
    // -----------------------------------------

    let schema_cols = meta.get_schema_columns(table_id)?;
    let schema = Schema::new(schema_cols.to_vec())?;

    let mut plan = PhysicalPlanner::build_plan(table_heap, bpm, &schema, &statement)?;
    let final_schema = plan.schema().clone();

    let mut result_rows = match statement.limit {
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
