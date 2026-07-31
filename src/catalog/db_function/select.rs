use crate::catalog::table::Table;
use crate::domain::id::ColumnId;
use crate::domain::{ColumnDef, DomainError, Row, Schema, SqlType, SqlValue};
use crate::execution::aggregate::AggregateFunc;
use crate::execution::operator::PhysicalOperator;
use crate::execution::sort::OrderByExpr;
use crate::execution::{
    AggregateOperator, FilterOperator, LimitOperator, ProjectionOperator, SeqScanOperator,
    SortOperator,
};
use crate::expr::Expr;

/// Pernyataan Query SELECT
#[derive(Debug, Clone)]
pub struct SelectStmt {
    /// Proyeksi ekspresi (misal: col1, col2, atau ekspresi matematika)
    pub projection: Vec<Expr>,
    /// Predikat penyaringan (WHERE)
    pub selection: Option<Expr>,
    /// Kolom GROUP BY
    pub group_by: Vec<ColumnId>,
    /// Fungsi Agregat (COUNT, SUM, AVG, MIN, MAX)
    pub aggregates: Vec<AggregateFunc>,
    /// Aturan pengurutan (ORDER BY)
    pub order_by: Vec<OrderByExpr>,
    /// Batas baris (LIMIT)
    pub limit: Option<usize>,
    /// Pergeseran baris awal (OFFSET)
    pub offset: usize,
}

/// Hasil dari eksekusi Query SELECT
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub schema: Schema,
    pub rows: Vec<Row>,
}

/// Eksekutor utama SELECT dengan merangkai Physical Operator Tree
pub(crate) fn execute_select(table: &Table, stmt: SelectStmt) -> Result<QueryResult, DomainError> {
    let base_schema = table.schema().clone();
    let base_rows = table.rows().to_vec();

    // 1. Leaf Node: SeqScanOperator
    let mut plan: Box<dyn PhysicalOperator> =
        Box::new(SeqScanOperator::new(base_schema, base_rows));

    // 2. Filter Node (WHERE)
    if let Some(predicate) = stmt.selection {
        plan = Box::new(FilterOperator::new(plan, predicate));
    }

    // 3. Aggregate Node (GROUP BY & Aggregations)
    if !stmt.group_by.is_empty() || !stmt.aggregates.is_empty() {
        let agg_output_schema =
            build_aggregate_schema(plan.schema(), &stmt.group_by, &stmt.aggregates)?;
        plan = Box::new(AggregateOperator::new(
            plan,
            stmt.group_by,
            stmt.aggregates,
            agg_output_schema,
        ));
    }

    // 4. Sort Node (ORDER BY)
    if !stmt.order_by.is_empty() {
        plan = Box::new(SortOperator::new(plan, stmt.order_by));
    }

    // 5. Limit / Offset Node
    if stmt.limit.is_some() || stmt.offset > 0 {
        plan = Box::new(LimitOperator::new(plan, stmt.limit, stmt.offset));
    }

    // 6. Projection Node (SELECT expressions)
    let final_schema = build_projection_schema(plan.schema(), &stmt.projection)?;
    plan = Box::new(ProjectionOperator::new(
        plan,
        stmt.projection,
        final_schema.clone(),
    ));

    // 7. Pull Rows dari Volcano Iterator Tree
    let mut result_rows = Vec::new();
    while let Some(row) = plan.next()? {
        result_rows.push(row);
    }

    Ok(QueryResult {
        schema: final_schema,
        rows: result_rows,
    })
}

/// Helper pembangun skema output untuk Aggregate Operator
fn build_aggregate_schema(
    child_schema: &Schema,
    group_by: &[ColumnId],
    aggregates: &[AggregateFunc],
) -> Result<Schema, DomainError> {
    let mut cols = Vec::new();

    // Kolom group by dipertahankan di skema hasil
    for &col_id in group_by {
        let col_def = child_schema.get_column_by_id(col_id).ok_or_else(|| {
            DomainError::EvaluationError(format!(
                "Kolom GROUP BY dengan ID {:?} tidak ditemukan",
                col_id
            ))
        })?;
        cols.push(col_def.clone());
    }

    // Kolom hasil agregasi ditambahkan
    for (i, agg) in aggregates.iter().enumerate() {
        let name = match agg {
            AggregateFunc::Count(_) => format!("count_{i}"),
            AggregateFunc::Sum(_) => format!("sum_{i}"),
            AggregateFunc::Avg(_) => format!("avg_{i}"),
            AggregateFunc::Min(_) => format!("min_{i}"),
            AggregateFunc::Max(_) => format!("max_{i}"),
        };
        // Agregat seperti SUM/COUNT umumnya bertipe Int atau Float
        cols.push(ColumnDef::new(
            ColumnId(9900 + i as u32),
            name,
            SqlType::Float,
        ));
    }

    Schema::new(cols)
}

/// Helper pembangun skema output untuk Projection Operator
fn build_projection_schema(
    child_schema: &Schema,
    projection: &[Expr],
) -> Result<Schema, DomainError> {
    let mut cols = Vec::with_capacity(projection.len());

    for (i, expr) in projection.iter().enumerate() {
        // Penentuan tipe data sederhana untuk kolom hasil proyeksi
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

            _ => SqlType::Text, // Default fallback untuk ekspresi kompleks
        };

        cols.push(ColumnDef::new(
            ColumnId(8800 + i as u32),
            format!("col_{i}"),
            col_type,
        ));
    }

    Schema::new(cols)
}
