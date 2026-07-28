use super::domain_error::DomainError;
use super::row::Row;
use super::schema::Schema;
use std::sync::{Arc, RwLock};

/// Representasi tabel dalam memori (In-Memory Table)
#[derive(Debug, Clone)]
pub struct Table {
    name: String,
    schema: Schema,
    // Menggunakan Arc<RwLock> internal agar pembacaan (SELECT) bisa concurrent,
    // namun penulisan (INSERT) aman dari race condition.
    rows: Arc<RwLock<Vec<Row>>>,
}

impl Table {
    pub fn new(name: impl Into<String>, schema: Schema) -> Self {
        Self {
            name: name.into(),
            schema,
            rows: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Menambahkan baris data ke dalam tabel dengan validasi skema
    pub fn insert(&self, row: Row) -> Result<(), DomainError> {
        // Validasi tipe data dan NULL constraint sebelum dimasukkan
        self.schema.validate_row(row.values())?;

        let mut rows = self.rows.write().map_err(|_| {
            DomainError::EvaluationError("Gagal mendapatkan write lock pada tabel".into())
        })?;

        rows.push(row);
        Ok(())
    }

    /// Menambahkan banyak baris data sekaligus
    pub fn insert_many(&self, new_rows: Vec<Row>) -> Result<usize, DomainError> {
        for row in &new_rows {
            self.schema.validate_row(row.values())?;
        }

        let mut rows = self.rows.write().map_err(|_| {
            DomainError::EvaluationError("Gagal mendapatkan write lock pada tabel".into())
        })?;

        let count = new_rows.len();
        rows.extend(new_rows);
        Ok(count)
    }

    /// Mengambil salinan snapshot dari seluruh baris data (Table Scan)
    pub fn scan(&self) -> Result<Vec<Row>, DomainError> {
        let rows = self.rows.read().map_err(|_| {
            DomainError::EvaluationError("Gagal mendapatkan read lock pada tabel".into())
        })?;

        Ok(rows.clone())
    }
}
