use serde::{Deserialize, Serialize};

use super::domain_error::DomainError;
use super::schema::Schema;
use super::sql_value::SqlValue;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Row {
    values: Vec<SqlValue>,
}

impl Row {
    pub fn new(values: Vec<SqlValue>) -> Self {
        Self { values }
    }

    pub fn values(&self) -> &[SqlValue] {
        &self.values
    }

    /// Mengambil nilai berdasarkan posisi indeks kolom
    pub fn get_by_index(&self, index: usize) -> Option<&SqlValue> {
        self.values.get(index)
    }

    /// Mengambil nilai berdasarkan nama kolom menggunakan bantuan Schema
    pub fn get_by_name<'a>(
        &'a self,
        schema: &Schema,
        col_name: &str,
    ) -> Result<&'a SqlValue, DomainError> {
        let idx = schema.index_of(col_name).ok_or_else(|| {
            DomainError::EvaluationError(format!("Kolom '{col_name}' tidak ditemukan pada skema"))
        })?;

        self.get_by_index(idx).ok_or_else(|| {
            DomainError::EvaluationError(format!("Data pada indeks {idx} tidak ditemukan"))
        })
    }

    /// Mengonsumsi Row dan mengembalikan inner Vec<SqlValue>
    pub fn into_values(self) -> Vec<SqlValue> {
        self.values
    }

    /// Mengambil dan mengeluarkan nilai pada indeks tertentu (memindahkan ownership)
    pub fn remove(&mut self, index: usize) -> Option<SqlValue> {
        if index < self.values.len() {
            Some(self.values.remove(index))
        } else {
            None
        }
    }
}

impl From<Vec<SqlValue>> for Row {
    fn from(values: Vec<SqlValue>) -> Self {
        Self { values }
    }
}

impl FromIterator<SqlValue> for Row {
    fn from_iter<T: IntoIterator<Item = SqlValue>>(iter: T) -> Self {
        Self {
            values: iter.into_iter().collect(),
        }
    }
}

use std::ops::Index;

// Contoh penggunaan:
// let val = &row[0];
impl Index<usize> for Row {
    type Output = SqlValue;

    fn index(&self, index: usize) -> &Self::Output {
        &self.values[index]
    }
}
