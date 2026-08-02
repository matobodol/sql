//! Physical operator untuk eksekusi fungsi agregasi SQL (`COUNT`, `SUM`, `AVG`, `MIN`, `MAX`)
//! serta pengelompokan baris data (`GROUP BY`).

use crate::domain::id::ColumnId;
use crate::domain::{DomainError, Row, Schema, SqlValue};
use crate::execution::operator::PhysicalOperator;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::vec::IntoIter;

/// Jenis-jenis fungsi agregasi yang didukung oleh engine SQL.
#[derive(Debug, Clone, PartialEq)]
pub enum AggregateFunc {
    /// Menghitung jumlah baris/nilai yang tidak NULL. `None` merepresentasikan `COUNT(*)`.
    Count(Option<ColumnId>),
    /// Menjumlahkan seluruh nilai pada kolom tertentu.
    Sum(ColumnId),
    /// Menghitung rata-rata nilai pada kolom tertentu.
    Avg(ColumnId),
    /// Mencari nilai minimum pada kolom tertentu.
    Min(ColumnId),
    /// Mencari nilai maksimum pada kolom tertentu.
    Max(ColumnId),
}

/// Akumulator stateful untuk memelihara status agregasi pada satu grup selama pemrosesan stream.
#[derive(Debug, Clone)]
pub enum Accumulator {
    /// Akumulator untuk fungsi `COUNT`.
    Count(i64),
    /// Akumulator untuk fungsi `SUM`.
    Sum(SqlValue),
    /// Akumulator untuk fungsi `AVG`, menyimpan akumulasi jumlah total dan total frekuensi nilai.
    Avg { sum: SqlValue, count: i64 },
    /// Akumulator untuk fungsi `MIN`.
    Min(Option<SqlValue>),
    /// Akumulator untuk fungsi `MAX`.
    Max(Option<SqlValue>),
}

impl Accumulator {
    /// Membuat akumulator baru yang diinisialisasi berdasarkan `AggregateFunc`.
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

    /// Memperbarui status akumulator dengan nilai baru dari baris data yang sedang diproses.
    ///
    /// Aturan SQL: Nilai `NULL` diabaikan oleh sebagian besar fungsi agregasi.
    pub fn update(&mut self, val: &SqlValue) -> Result<(), DomainError> {
        match self {
            Accumulator::Count(c) => {
                if !val.is_null() {
                    *c += 1;
                }
            }
            Accumulator::Sum(acc_val) => {
                if !val.is_null() {
                    *acc_val = acc_val.add(val)?; // Menggunakan operasi penambahan SSOT pada SqlValue
                }
            }
            Accumulator::Avg { sum, count } => {
                if !val.is_null() {
                    *sum = sum.add(val)?; // Menggunakan operasi penambahan SSOT pada SqlValue
                    *count += 1;
                }
            }
            Accumulator::Min(acc_val) => {
                if !val.is_null() {
                    match acc_val {
                        Some(cur) => {
                            if val.cmp(cur) == Ordering::Less {
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
                            if val.cmp(cur) == Ordering::Greater {
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

    /// Mengevaluasi dan menghasilkan nilai `SqlValue` akhir dari status akumulator.
    pub fn evaluate(&self) -> SqlValue {
        match self {
            Accumulator::Count(c) => SqlValue::Int(*c),
            Accumulator::Sum(val) => val.clone(),
            Accumulator::Avg { sum, count } => {
                if *count == 0 || sum.is_null() {
                    SqlValue::Null
                } else {
                    // Menggunakan method .div() SSOT milik SqlValue
                    sum.div(&SqlValue::Int(*count)).unwrap_or(SqlValue::Null)
                }
            }
            Accumulator::Min(val) => val.clone().unwrap_or(SqlValue::Null),
            Accumulator::Max(val) => val.clone().unwrap_or(SqlValue::Null),
        }
    }
}

/// Physical operator yang bertanggung jawab mengeksekusi operasi agregasi dan `GROUP BY`.
pub struct AggregateOperator {
    /// Operator anak yang memasok input stream data.
    input: Box<dyn PhysicalOperator>,
    /// Daftar ID kolom yang digunakan sebagai kunci pengelompokan (`GROUP BY`).
    group_by_cols: Vec<ColumnId>,
    /// Daftar fungsi agregasi yang akan dihitung per grup.
    aggregates: Vec<AggregateFunc>,
    /// Skema keluaran dari operator agregasi ini.
    output_schema: Schema,
    /// Cache baris hasil agregasi yang siap diteruskan ke pipeline berikutnya.
    aggregated_rows: Option<IntoIter<Row>>,
}

impl AggregateOperator {
    /// Membuat instance `AggregateOperator` baru.
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

    /// Membaca seluruh data input stream, melakukan proses pengelompokan kunci (Hash-based Aggregation),
    /// dan mengevaluasi seluruh nilai agregat.
    fn fetch_and_aggregate(&mut self) -> Result<(), DomainError> {
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
        };

        let mut groups: HashMap<Vec<SqlValue>, Vec<Accumulator>> = HashMap::new();
        let count_star_dummy = SqlValue::Int(1);

        while let Some(row) = self.input.next()? {
            let group_key: Vec<SqlValue> =
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
