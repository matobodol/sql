use ordered_float::OrderedFloat;

use crate::domain::id::ColumnId;
use crate::domain::{DomainError, Row, Schema, SqlValue};
use crate::execution::operator::PhysicalOperator;
use std::collections::HashMap;
use std::vec::IntoIter;

#[derive(Debug, Clone, PartialEq)]
pub enum AggregateFunc {
    Count(Option<ColumnId>),
    Sum(ColumnId),
    Avg(ColumnId),
    Min(ColumnId),
    Max(ColumnId),
}

#[derive(Debug, Clone)]
pub enum Accumulator {
    Count(i64),
    Sum(SqlValue),
    Avg { sum: SqlValue, count: i64 },
    Min(Option<SqlValue>),
    Max(Option<SqlValue>),
}

impl Accumulator {
    pub fn new(func: &AggregateFunc) -> Self {
        match func {
            AggregateFunc::Count(_) => Accumulator::Count(0),
            AggregateFunc::Sum(_) => Accumulator::Sum(SqlValue::Null),
            AggregateFunc::Avg(_) => Accumulator::Avg {
                sum: SqlValue::Null,
                count: 0,
            },
            AggregateFunc::Min(_) => Accumulator::Min(None),
            AggregateFunc::Max(_) => Accumulator::Max(None),
        }
    }

    pub fn update(&mut self, val: &SqlValue) -> Result<(), DomainError> {
        match self {
            Accumulator::Count(c) => {
                if !val.is_null() {
                    *c += 1;
                }
            }
            Accumulator::Sum(acc_val) => {
                if !val.is_null() {
                    *acc_val = add_sql_values(acc_val, val)?;
                }
            }
            Accumulator::Avg { sum, count } => {
                if !val.is_null() {
                    *sum = add_sql_values(sum, val)?;
                    *count += 1;
                }
            }
            Accumulator::Min(acc_val) => {
                if !val.is_null() {
                    match acc_val {
                        Some(cur) => {
                            if val.lt(cur).is_true() {
                                *acc_val = Some(val.clone());
                            }
                        }
                        None => *acc_val = Some(val.clone()),
                    }
                }
            }
            Accumulator::Max(acc_val) => {
                if !val.is_null() {
                    match acc_val {
                        Some(cur) => {
                            if val.gt(cur).is_true() {
                                *acc_val = Some(val.clone());
                            }
                        }
                        None => *acc_val = Some(val.clone()),
                    }
                }
            }
        }
        Ok(())
    }

    pub fn evaluate(&self) -> SqlValue {
        match self {
            Accumulator::Count(c) => SqlValue::Int(*c),
            Accumulator::Sum(val) => val.clone(),
            Accumulator::Avg { sum, count } => {
                if *count == 0 {
                    SqlValue::Null
                } else {
                    match sum {
                        SqlValue::Int(s) => {
                            SqlValue::Float(OrderedFloat::from((*s as f64) / (*count as f64)))
                        }
                        SqlValue::Float(s) => SqlValue::Float(*s / (*count as f64)),
                        _ => SqlValue::Null,
                    }
                }
            }
            Accumulator::Min(val) => val.clone().unwrap_or(SqlValue::Null),
            Accumulator::Max(val) => val.clone().unwrap_or(SqlValue::Null),
        }
    }
}

fn add_sql_values(a: &SqlValue, b: &SqlValue) -> Result<SqlValue, DomainError> {
    match (a, b) {
        (SqlValue::Null, val) => Ok(val.clone()),
        (val, SqlValue::Null) => Ok(val.clone()),
        (SqlValue::Int(x), SqlValue::Int(y)) => Ok(SqlValue::Int(x + y)),
        (SqlValue::Float(x), SqlValue::Float(y)) => Ok(SqlValue::Float(x + y)),
        (SqlValue::Int(x), SqlValue::Float(y)) => {
            Ok(SqlValue::Float(OrderedFloat::from(*x as f64) + y))
        }
        (SqlValue::Float(x), SqlValue::Int(y)) => Ok(SqlValue::Float(x + *y as f64)),
        _ => Err(DomainError::EvaluationError(
            "Tipe data tidak cocok untuk penjumlahan agregasi".into(),
        )),
    }
}

pub struct AggregateOperator {
    input: Box<dyn PhysicalOperator>,
    group_by_cols: Vec<ColumnId>,
    aggregates: Vec<AggregateFunc>,
    output_schema: Schema,
    aggregated_rows: Option<IntoIter<Row>>,
}

impl AggregateOperator {
    pub fn new(
        input: Box<dyn PhysicalOperator>,
        group_by_cols: Vec<ColumnId>,
        aggregates: Vec<AggregateFunc>,
        output_schema: Schema,
    ) -> Self {
        Self {
            input,
            group_by_cols,
            aggregates,
            output_schema,
            aggregated_rows: None,
        }
    }

    fn fetch_and_aggregate(&mut self) -> Result<(), DomainError> {
        // 1. Pre-calculate seluruh indeks kolom di awal dan LEPAS borrow schema!
        let (group_indices, agg_target_indices) = {
            let child_schema = self.input.schema();

            let g_indices: Vec<usize> = self
                .group_by_cols
                .iter()
                .map(|col_id| {
                    child_schema.index_of_id(*col_id).ok_or_else(|| {
                        DomainError::EvaluationError("Kolom Group By tidak ditemukan".into())
                    })
                })
                .collect::<Result<_, _>>()?;

            let a_indices: Vec<Option<usize>> = self
                .aggregates
                .iter()
                .map(|func| match func {
                    AggregateFunc::Count(Some(col_id))
                    | AggregateFunc::Sum(col_id)
                    | AggregateFunc::Avg(col_id)
                    | AggregateFunc::Min(col_id)
                    | AggregateFunc::Max(col_id) => {
                        let idx = child_schema.index_of_id(*col_id).ok_or_else(|| {
                            DomainError::EvaluationError("Kolom agregat tidak ditemukan".into())
                        })?;
                        Ok(Some(idx))
                    }
                    AggregateFunc::Count(None) => Ok(None),
                })
                .collect::<Result<_, _>>()?;

            (g_indices, a_indices)
        }; // 👈 Borrow child_schema berakhir di sini!

        // 2. State penampung agregat
        let mut groups: HashMap<Vec<SqlValue>, Vec<Accumulator>> = HashMap::new();

        // 3. Sekarang aman untuk memanggil self.input.next()? secara mutable
        while let Some(row) = self.input.next()? {
            let group_key: Vec<SqlValue> =
                group_indices.iter().map(|&idx| row[idx].clone()).collect();

            let accumulators = groups
                .entry(group_key)
                .or_insert_with(|| self.aggregates.iter().map(Accumulator::new).collect());

            for (acc, target_idx) in accumulators.iter_mut().zip(&agg_target_indices) {
                let dummy_one = SqlValue::Int(1);
                let val = match target_idx {
                    Some(idx) => &row[*idx],
                    None => &dummy_one, // Nilai dummy untuk COUNT(*)
                };
                acc.update(val)?;
            }
        }

        // 4. Bangun final rows
        let mut final_rows = Vec::new();
        for (group_key, accumulators) in groups {
            let mut row_values = group_key;
            for acc in accumulators {
                row_values.push(acc.evaluate());
            }
            final_rows.push(Row::new(row_values));
        }

        self.aggregated_rows = Some(final_rows.into_iter());
        Ok(())
    }
}

impl PhysicalOperator for AggregateOperator {
    fn schema(&self) -> &Schema {
        &self.output_schema
    }

    fn next(&mut self) -> Result<Option<Row>, DomainError> {
        if self.aggregated_rows.is_none() {
            self.fetch_and_aggregate()?;
        }

        Ok(self.aggregated_rows.as_mut().unwrap().next())
    }
}
