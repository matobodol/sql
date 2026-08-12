use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::vec::IntoIter;

use crate::execution::operator::PhysicalOperator;
use crate::{BufferPoolManager, ColumnId, DomainError, Row, RowId, Schema, ValueType};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    Sum(ValueType),
    Avg { sum: ValueType, count: i64 },
    Min(Option<ValueType>),
    Max(Option<ValueType>),
}

impl Accumulator {
    pub fn new(func: &AggregateFunc) -> Self {
        match func {
            AggregateFunc::Count(_) => Accumulator::Count(0),
            AggregateFunc::Sum(_) => Accumulator::Sum(ValueType::Null),
            AggregateFunc::Avg(_) => Accumulator::Avg {
                sum: ValueType::Null,
                count: 0,
            },
            AggregateFunc::Min(_) => Accumulator::Min(None),
            AggregateFunc::Max(_) => Accumulator::Max(None),
        }
    }

    #[inline]
    pub fn update(&mut self, val: &ValueType) -> Result<(), DomainError> {
        if val.is_null() {
            return Ok(());
        }

        match self {
            Accumulator::Count(c) => {
                *c += 1;
            }
            Accumulator::Sum(acc_val) => {
                *acc_val = acc_val.add(val)?;
            }
            Accumulator::Avg { sum, count } => {
                *sum = sum.add(val)?;
                *count += 1;
            }
            Accumulator::Min(acc_val) => match acc_val {
                Some(cur) => {
                    if val.cmp(cur) == Ordering::Less {
                        *acc_val = Some(val.clone());
                    }
                }
                None => *acc_val = Some(val.clone()),
            },
            Accumulator::Max(acc_val) => match acc_val {
                Some(cur) => {
                    if val.cmp(cur) == Ordering::Greater {
                        *acc_val = Some(val.clone());
                    }
                }
                None => *acc_val = Some(val.clone()),
            },
        }
        Ok(())
    }

    pub fn evaluate(&self) -> ValueType {
        match self {
            Accumulator::Count(c) => ValueType::Int(*c),
            Accumulator::Sum(val) => val.clone(),
            Accumulator::Avg { sum, count } => {
                if *count == 0 || sum.is_null() {
                    ValueType::Null
                } else {
                    sum.div(&ValueType::Int(*count)).unwrap_or(ValueType::Null)
                }
            }
            Accumulator::Min(val) => val.clone().unwrap_or(ValueType::Null),
            Accumulator::Max(val) => val.clone().unwrap_or(ValueType::Null),
        }
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

    fn fetch_and_aggregate(&mut self, bpm: &mut BufferPoolManager) -> Result<(), DomainError> {
        let child_schema = self.input.schema();

        let mut group_indices = Vec::with_capacity(self.group_by_cols.len());
        for col_id in &self.group_by_cols {
            let idx = child_schema.index_of_id(*col_id).ok_or_else(|| {
                DomainError::EvaluationError("Kolom Group By tidak ditemukan".into())
            })?;
            group_indices.push(idx);
        }

        let mut agg_target_indices = Vec::with_capacity(self.aggregates.len());
        for func in &self.aggregates {
            let idx_opt = match func {
                AggregateFunc::Count(Some(col_id))
                | AggregateFunc::Sum(col_id)
                | AggregateFunc::Avg(col_id)
                | AggregateFunc::Min(col_id)
                | AggregateFunc::Max(col_id) => {
                    let idx = child_schema.index_of_id(*col_id).ok_or_else(|| {
                        DomainError::EvaluationError("Kolom agregat tidak ditemukan".into())
                    })?;
                    Some(idx)
                }
                AggregateFunc::Count(None) => None,
            };
            agg_target_indices.push(idx_opt);
        }

        let mut groups: HashMap<Vec<ValueType>, Vec<Accumulator>> = HashMap::new();
        let count_star_dummy = ValueType::Int(1);

        while let Some(row) = self.input.next(bpm)? {
            let group_key: Vec<ValueType> =
                group_indices.iter().map(|&idx| row[idx].clone()).collect();

            let accumulators = groups
                .entry(group_key)
                .or_insert_with(|| self.aggregates.iter().map(Accumulator::new).collect());

            for (acc, target_idx) in accumulators.iter_mut().zip(&agg_target_indices) {
                let val = match target_idx {
                    Some(idx) => &row[*idx],
                    None => &count_star_dummy,
                };
                acc.update(val)?;
            }
        }

        let mut final_rows = Vec::with_capacity(groups.len());
        for (group_key, accumulators) in groups {
            let mut row_values = group_key;
            for acc in accumulators {
                row_values.push(acc.evaluate());
            }
            final_rows.push(Row::with_id(RowId::from(0u64), row_values));
        }

        self.aggregated_rows = Some(final_rows.into_iter());
        Ok(())
    }
}

impl PhysicalOperator for AggregateOperator {
    #[inline]
    fn schema(&self) -> &Schema {
        &self.output_schema
    }

    #[inline]
    fn next(&mut self, bpm: &mut BufferPoolManager) -> Result<Option<Row>, DomainError> {
        if self.aggregated_rows.is_none() {
            self.fetch_and_aggregate(bpm)?;
        }

        if let Some(iter) = &mut self.aggregated_rows {
            Ok(iter.next())
        } else {
            Ok(None)
        }
    }
}
