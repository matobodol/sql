use std::sync::Arc;

use crate::execution::operator::PhysicalOperator;
use crate::execution::{
    AggregateOperator, FilterOperator, IndexScanOperator, LimitOperator, ProjectionOperator,
    SeqScanOperator, SortOperator,
};
use crate::query_logic::dml_action::try_index_scan;
use crate::{
    AggregateFunc, Column, ColumnId, DataType, DomainError, Expr, OrderByExpr, Schema,
    TableStorage, ValueType,
};

#[derive(Debug, Clone, PartialEq)]
pub struct SelectStmt {
    pub projection: Vec<Expr>,
    pub selection: Option<Expr>,
    pub group_by: Vec<ColumnId>,
    pub aggregates: Vec<AggregateFunc>,
    pub order_by: Vec<OrderByExpr>,
    pub limit: Option<usize>,
    pub offset: usize,
}

pub struct PhysicalPlanner;

impl PhysicalPlanner {
    pub fn build_plan(
        table: &TableStorage,
        schema: &Schema,
        stmt: &SelectStmt,
    ) -> Result<Box<dyn PhysicalOperator>, DomainError> {
        // ------------------------------------------------------------------
        // LANGKAH 1: Inisialisasi Root Scan Operator (IndexScan vs SeqScan)
        // ------------------------------------------------------------------
        let (mut plan, is_index_scan): (Box<dyn PhysicalOperator>, bool) =
            if let Some(candidate_ids) = try_index_scan(table, schema, stmt.selection.as_ref()) {
                (
                    Box::new(IndexScanOperator::new(table, schema.clone(), candidate_ids)),
                    true,
                )
            } else {
                let rows_arc = table.row_store().rows_arc();
                (
                    Box::new(SeqScanOperator::new(rows_arc, schema.clone())),
                    false,
                )
            };

        // ------------------------------------------------------------------
        // LANGKAH 2: Tumpuk FilterOperator (WHERE) jika bukan index scan murni
        // ------------------------------------------------------------------
        if let Some(ref predicate) = stmt.selection {
            if !is_index_scan {
                plan = Box::new(FilterOperator::new(plan, predicate.clone())?);
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
            )?);
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

fn build_aggregate_schema(
    child_schema: &Schema,
    group_by: &[ColumnId],
    aggregates: &[AggregateFunc],
) -> Result<Schema, DomainError> {
    let mut cols = Vec::new();

    for &col_id in group_by {
        let col_def = child_schema.get_column_by_id(col_id).ok_or_else(|| {
            DomainError::eval_error(format!(
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

        cols.push(Column::new(
            ColumnId(9900 + i as u32),
            name,
            DataType::Float,
        ));
    }

    Schema::new(cols)
}

fn build_projection_schema(
    child_schema: &Schema,
    projection: &[Expr],
) -> Result<Schema, DomainError> {
    let mut cols = Vec::with_capacity(projection.len());

    for (i, expr) in projection.iter().enumerate() {
        let (col_name, col_type) = match expr {
            Expr::Column(name) => {
                let def = child_schema
                    .get_column_by_name(name)
                    .ok_or_else(|| DomainError::ColumnNotFound(Arc::from(name.as_str())))?;
                (name.clone(), def.sql_type.clone())
            }
            Expr::Literal(val) => {
                let t = match val {
                    ValueType::Null => DataType::Int,
                    ValueType::Int(_) => DataType::Int,
                    ValueType::Float(_) => DataType::Float,
                    ValueType::Text(_) => DataType::Text,
                    ValueType::Bool(_) => DataType::Bool,
                    ValueType::Bytes(_) => DataType::Bytes,
                    ValueType::Timestamp(_) => DataType::Timestamp,
                    ValueType::Date(_) => DataType::Date,
                    ValueType::Time(_) => DataType::Time,
                    ValueType::Enum { type_name, .. } => DataType::Enum {
                        name: type_name.to_string(),
                        variants: vec![],
                    },
                    ValueType::Custom { type_name, .. } => DataType::Custom(type_name.to_string()),
                };
                (format!("col_{i}"), t)
            }
            _ => (format!("col_{i}"), DataType::Text),
        };

        cols.push(Column::new(ColumnId(8800 + i as u32), col_name, col_type));
    }

    Schema::new(cols)
}
