use super::domain_error::DomainError;
use super::schema::Schema;
use super::sql_type::SqlValue;

#[derive(Debug, Clone, PartialEq)]
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
}
