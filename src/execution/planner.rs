use std::sync::Arc;

use crate::catalog::database::Database;
use crate::catalog::dml_action::try_index_scan; // Import helper dari dml_action
use crate::catalog::dql_action::SelectStmt;
use crate::domain::DomainError;
use crate::execution::operator::PhysicalOperator;
use crate::execution::{
    AggregateOperator, FilterOperator, LimitOperator, MemoryRowIterator, ProjectionOperator,
    SeqScanOperator, SortOperator,
};
use crate::{
    AggregateFunc, ColumnDef, ColumnId, Expr, IndexScanOperator, Schema, SqlType, SqlValue,
};

pub struct PhysicalPlanner;

impl PhysicalPlanner {
    pub fn build_plan(
        db: &Database,
        table_name: &str,
        stmt: &SelectStmt,
    ) -> Result<Box<dyn PhysicalOperator>, DomainError> {
        let table = db.get_table(table_name)?;
        let schema = table.schema();

        // ------------------------------------------------------------------
        // LANGKAH 1: Inisialisasi Root Scan Operator (IndexScan vs SeqScan)
        // ------------------------------------------------------------------
        // ------------------------------------------------------------------
        // LANGKAH 1 & 2: Root Scan & Filtering
        // ------------------------------------------------------------------
        let (mut plan, is_index_scan): (Box<dyn PhysicalOperator>, bool) =
            if let Some(candidate_ids) = try_index_scan(table, stmt.selection.as_ref()) {
                (Box::new(IndexScanOperator::new(table, candidate_ids)), true)
            } else {
                let rows_arc = Arc::new(table.rows().to_vec());
                let memory_iter = Box::new(MemoryRowIterator::new(rows_arc));
                (
                    Box::new(SeqScanOperator::new(memory_iter, schema.clone())),
                    false,
                )
            };

        // ------------------------------------------------------------------
        // LANGKAH 2: Tumpuk FilterOperator (WHERE) jika ada predikat
        // ------------------------------------------------------------------
        // Hanya tambahkan FilterOperator jika BUtuh (misal Sequential Scan)
        if let Some(ref predicate) = stmt.selection {
            if !is_index_scan {
                plan = Box::new(FilterOperator::new(plan, predicate.clone()));
            }
        }

        // ------------------------------------------------------------------
        // LANGKAH 3: Tumpuk AggregateOperator (GROUP BY & Aggregates)
        // ------------------------------------------------------------------
        if !stmt.group_by.is_empty() || !stmt.aggregates.is_empty() {
            let agg_schema =
                build_aggregate_schema(plan.schema(), &stmt.group_by, &stmt.aggregates)?;
            plan = Box::new(AggregateOperator::new(
                plan,
                stmt.group_by.clone(),
                stmt.aggregates.clone(),
                agg_schema,
            ));
        }

        // ------------------------------------------------------------------
        // LANGKAH 4: Tumpuk SortOperator (ORDER BY)
        // ------------------------------------------------------------------
        if !stmt.order_by.is_empty() {
            plan = Box::new(SortOperator::new(plan, stmt.order_by.clone()));
        }

        // ------------------------------------------------------------------
        // LANGKAH 5: Tumpuk ProjectionOperator (SELECT Expressions)
        // ------------------------------------------------------------------
        if !stmt.projection.is_empty() {
            let proj_schema = build_projection_schema(plan.schema(), &stmt.projection)?;
            plan = Box::new(ProjectionOperator::new(
                plan,
                stmt.projection.clone(),
                proj_schema,
            ));
        }

        // ------------------------------------------------------------------
        // LANGKAH 6: Tumpuk LimitOperator (OFFSET & LIMIT)
        // ------------------------------------------------------------------
        if stmt.limit.is_some() || stmt.offset > 0 {
            plan = Box::new(LimitOperator::new(plan, stmt.limit, stmt.offset));
        }

        Ok(plan)
    }
}

// ----------------------------------------------------------------------
// HELPER FUNCTIONS FOR SCHEMA BUILDING
// ----------------------------------------------------------------------

/// Helper internal untuk membangun skema keluaran dari operator agregasi (`AggregateOperator`).
fn build_aggregate_schema(
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

/// Helper internal untuk membangun skema keluaran dari operator proyeksi (`ProjectionOperator`).
fn build_projection_schema(
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
                SqlValue::Null => SqlType::Int,
                SqlValue::Int(_) => SqlType::Int,
                SqlValue::Float(_) => SqlType::Float,
                SqlValue::Text(_) => SqlType::Text,
                SqlValue::Bool(_) => SqlType::Bool,
                SqlValue::Bytes(_) => SqlType::Bytes,
                SqlValue::Timestamp(_) => SqlType::Timestamp,
                SqlValue::Date(_) => SqlType::Date,
                SqlValue::Time(_) => SqlType::Time,

                // Inferensi Enum & Custom dari SqlValue
                SqlValue::Enum { type_name, .. } => SqlType::Enum {
                    name: type_name.clone(),
                    variants: vec![],
                },
                SqlValue::Custom { type_name, .. } => SqlType::Custom(type_name.clone()),
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
