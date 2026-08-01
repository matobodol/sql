use std::sync::Arc;

use crate::catalog::database::Database;
use crate::domain::id::ColumnId;
use crate::domain::{ColumnDef, DomainError, Row, Schema, SqlType, SqlValue};
use crate::execution::aggregate::AggregateFunc;
use crate::execution::operator::PhysicalOperator;
use crate::execution::sort::OrderByExpr;
use crate::expr::Expr;
use crate::{
    AggregateOperator, FilterOperator, LimitOperator, MemoryRowIterator, ProjectionOperator,
    SeqScanOperator, SortOperator,
};

/// Pernyataan Query SELECT (Data Query Language - DQL).
#[derive(Debug, Clone)]
pub struct SelectStmt {
    pub projection: Vec<Expr>,
    pub selection: Option<Expr>,
    pub group_by: Vec<ColumnId>,
    pub aggregates: Vec<AggregateFunc>,
    pub order_by: Vec<OrderByExpr>,
    pub limit: Option<usize>,
    pub offset: usize,
}

/// Hasil dari eksekusi Query SELECT.
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub schema: Schema,
    pub rows: Vec<Row>,
}

/// Menjalankan query SELECT dengan 100% Lazy Volcano Execution Pipeline.
pub fn execute_select(
    db: &Database,
    table_name: &str,
    stmt: SelectStmt,
) -> Result<QueryResult, DomainError> {
    let table = db.get_table(table_name)?;
    let schema = table.schema();

    // ------------------------------------------------------------------
    // LANGKAH 1: Inisialisasi Root Operator (SeqScan via MemoryRowIterator)
    // ------------------------------------------------------------------
    let rows_arc = Arc::new(table.rows().to_vec());
    let memory_iter = Box::new(MemoryRowIterator::new(rows_arc));

    let mut plan: Box<dyn PhysicalOperator> =
        Box::new(SeqScanOperator::new(memory_iter, schema.clone()));

    // ------------------------------------------------------------------
    // LANGKAH 2: Tumpuk FilterOperator (WHERE) jika ada predikat
    // ------------------------------------------------------------------
    if let Some(predicate) = stmt.selection {
        plan = Box::new(FilterOperator::new(plan, predicate));
    }

    // ------------------------------------------------------------------
    // LANGKAH 3: Tumpuk AggregateOperator (GROUP BY & Aggregates)
    // ------------------------------------------------------------------
    if !stmt.group_by.is_empty() || !stmt.aggregates.is_empty() {
        let agg_schema = build_aggregate_schema(plan.schema(), &stmt.group_by, &stmt.aggregates)?;
        plan = Box::new(AggregateOperator::new(
            plan,
            stmt.group_by,
            stmt.aggregates,
            agg_schema,
        ));
    }

    // ------------------------------------------------------------------
    // LANGKAH 4: Tumpuk SortOperator (ORDER BY) jika ada aturan urutan
    // ------------------------------------------------------------------
    if !stmt.order_by.is_empty() {
        plan = Box::new(SortOperator::new(plan, stmt.order_by));
    }

    // ------------------------------------------------------------------
    // LANGKAH 5: Tumpuk ProjectionOperator (SELECT Expressions)
    // ------------------------------------------------------------------
    if !stmt.projection.is_empty() {
        let proj_schema = build_projection_schema(plan.schema(), &stmt.projection)?;
        plan = Box::new(ProjectionOperator::new(plan, stmt.projection, proj_schema));
    }

    // ------------------------------------------------------------------
    // LANGKAH 6: Tumpuk LimitOperator (OFFSET & LIMIT)
    // ------------------------------------------------------------------
    if stmt.limit.is_some() || stmt.offset > 0 {
        plan = Box::new(LimitOperator::new(plan, stmt.limit, stmt.offset));
    }

    // ------------------------------------------------------------------
    // LANGKAH 7: Eksekusi Pipeline secara Lazy (Pull Data)
    // ------------------------------------------------------------------
    let final_schema = plan.schema().clone();
    let mut result_rows = Vec::new();

    while let Some(row) = plan.next()? {
        result_rows.push(row);
    }

    Ok(QueryResult {
        schema: final_schema,
        rows: result_rows,
    })
}

// ----------------------------------------------------------------------
// HELPER FUNCTIONS FOR SCHEMA BUILDING
// ----------------------------------------------------------------------

/// Helper internal pembangun skema output untuk Aggregate Operator.
pub(crate) fn build_aggregate_schema(
    child_schema: &Schema,
    group_by: &[ColumnId],
    aggregates: &[AggregateFunc],
) -> Result<Schema, DomainError> {
    let mut cols = Vec::new();

    for &col_id in group_by {
        let col_def = child_schema.get_column_by_id(col_id).ok_or_else(|| {
            DomainError::EvaluationError(format!(
                "Kolom GROUP BY dengan ID {:?} tidak ditemukan",
                col_id
            ))
        })?;
        cols.push(col_def.clone());
    }

    for (i, agg) in aggregates.iter().enumerate() {
        let name = match agg {
            AggregateFunc::Count(_) => format!("count_{i}"),
            AggregateFunc::Sum(_) => format!("sum_{i}"),
            AggregateFunc::Avg(_) => format!("avg_{i}"),
            AggregateFunc::Min(_) => format!("min_{i}"),
            AggregateFunc::Max(_) => format!("max_{i}"),
        };

        cols.push(ColumnDef::new(
            ColumnId(9900 + i as u32),
            name,
            SqlType::Float,
        ));
    }

    Schema::new(cols)
}

/// Helper internal pembangun skema output untuk Projection Operator.
pub(crate) fn build_projection_schema(
    child_schema: &Schema,
    projection: &[Expr],
) -> Result<Schema, DomainError> {
    let mut cols = Vec::with_capacity(projection.len());

    for (i, expr) in projection.iter().enumerate() {
        let col_type = match expr {
            Expr::Column(col_id) => {
                let def = child_schema.get_column_by_id(*col_id).ok_or_else(|| {
                    DomainError::EvaluationError(format!(
                        "Kolom ID {:?} tidak ditemukan dalam proyeksi",
                        col_id
                    ))
                })?;
                def.sql_type.clone()
            }
            Expr::Literal(val) => match val {
                SqlValue::Int(_) => SqlType::Int,
                SqlValue::Float(_) => SqlType::Float,
                SqlValue::Text(_) => SqlType::Text,
                SqlValue::Bool(_) => SqlType::Bool,
                SqlValue::Bytes(_) => SqlType::Bytes,
                SqlValue::Timestamp(_) => SqlType::Timestamp,
                SqlValue::Date(_) => SqlType::Date,
                SqlValue::Time(_) => SqlType::Time,
                SqlValue::Null => SqlType::Int,
            },
            _ => SqlType::Text,
        };

        cols.push(ColumnDef::new(
            ColumnId(8800 + i as u32),
            format!("col_{i}"),
            col_type,
        ));
    }

    Schema::new(cols)
}
