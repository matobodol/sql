use super::operator::PhysicalOperator;
use crate::domain::{DomainError, Expr, Row, Schema, SqlValue};
use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

#[derive(Debug, Clone)]
pub struct OrderByExpr {
    pub expr: Expr,
    pub order: SortOrder,
}

pub struct SortOperator {
    input: Box<dyn PhysicalOperator>,
    order_by: Vec<OrderByExpr>,
    // Buffer internal untuk menyimpan baris yang sudah diurutkan
    sorted_rows: Option<Vec<Row>>,
    cursor: usize,
}

impl SortOperator {
    pub fn new(input: Box<dyn PhysicalOperator>, order_by: Vec<OrderByExpr>) -> Self {
        Self {
            input,
            order_by,
            sorted_rows: None,
            cursor: 0,
        }
    }

    /// Helper internal untuk memuat seluruh baris dan mengurutkannya
    fn fetch_and_sort(&mut self) -> Result<(), DomainError> {
        let schema = self.input.schema().clone();
        let mut rows = Vec::new();

        // 1. Pull seluruh data (Blocking phase)
        while let Some(row) = self.input.next()? {
            rows.push(row);
        }

        // 2. Urutkan baris berdasarkan kriteria ORDER BY
        let order_by = &self.order_by;
        let mut sort_error: Option<DomainError> = None;

        rows.sort_by(|a, b| {
            if sort_error.is_some() {
                return Ordering::Equal;
            }

            for spec in order_by {
                let val_a = match spec.expr.eval(&schema, a) {
                    Ok(v) => v,
                    Err(e) => {
                        sort_error = Some(e);
                        return Ordering::Equal;
                    }
                };

                let val_b = match spec.expr.eval(&schema, b) {
                    Ok(v) => v,
                    Err(e) => {
                        sort_error = Some(e);
                        return Ordering::Equal;
                    }
                };

                let ord = compare_sql_values(&val_a, &val_b);
                if ord != Ordering::Equal {
                    return match spec.order {
                        SortOrder::Ascending => ord,
                        SortOrder::Descending => ord.reverse(),
                    };
                }
            }

            Ordering::Equal
        });

        if let Some(err) = sort_error {
            return Err(err);
        }

        self.sorted_rows = Some(rows);
        Ok(())
    }
}

impl PhysicalOperator for SortOperator {
    fn schema(&self) -> &Schema {
        self.input.schema()
    }

    fn next(&mut self) -> Result<Option<Row>, DomainError> {
        // Jika data belum dimuat dan diurutkan, lakukan pemuatan sekarang (Lazy Materialization)
        if self.sorted_rows.is_none() {
            self.fetch_and_sort()?;
        }

        let rows = self.sorted_rows.as_ref().unwrap();

        if self.cursor < rows.len() {
            let row = rows[self.cursor].clone();
            self.cursor += 1;
            Ok(Some(row))
        } else {
            Ok(None)
        }
    }
}

/// Helper perbandingan `SqlValue` untuk sorting
fn compare_sql_values(a: &SqlValue, b: &SqlValue) -> Ordering {
    match (a, b) {
        (SqlValue::Null, SqlValue::Null) => Ordering::Equal,
        (SqlValue::Null, _) => Ordering::Less, // NULL dianggap paling kecil
        (_, SqlValue::Null) => Ordering::Greater,
        (SqlValue::Int(x), SqlValue::Int(y)) => x.cmp(y),
        (SqlValue::Float(x), SqlValue::Float(y)) => x.partial_cmp(y).unwrap_or(Ordering::Equal),
        (SqlValue::Text(x), SqlValue::Text(y)) => x.cmp(y),
        (SqlValue::Bool(x), SqlValue::Bool(y)) => x.cmp(y),
        (SqlValue::Timestamp(x), SqlValue::Timestamp(y)) => x.cmp(y),
        (SqlValue::Date(x), SqlValue::Date(y)) => x.cmp(y),
        (SqlValue::Time(x), SqlValue::Time(y)) => x.cmp(y),
        _ => Ordering::Equal,
    }
}
