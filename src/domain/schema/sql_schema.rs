use crate::{DomainError, SqlValue, schema::ColumnDef};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Schema {
    columns: Vec<ColumnDef>,
}

impl Schema {
    pub fn new(columns: Vec<ColumnDef>) -> Self {
        Self { columns }
    }

    pub fn columns(&self) -> &[ColumnDef] {
        &self.columns
    }

    /// Mencari indeks posisi kolom berdasarkan nama
    pub fn index_of(&self, col_name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c.name == col_name)
    }

    /// Validasi apakah sebuah row sesuai dengan skema ini
    pub fn validate_row(&self, values: &[SqlValue]) -> Result<(), DomainError> {
        if values.len() != self.columns.len() {
            return Err(DomainError::EvaluationError(format!(
                "Jumlah kolom tidak sesuai: mengharapkan {}, ditemukan {}",
                self.columns.len(),
                values.len()
            )));
        }

        for (col, val) in self.columns.iter().zip(values.iter()) {
            if val == &SqlValue::Null {
                if !col.nullable {
                    return Err(DomainError::EvaluationError(format!(
                        "Kolom '{}' tidak boleh NULL",
                        col.name
                    )));
                }
            } else if !val.matches_type(&col.sql_type) {
                return Err(DomainError::TypeMismatch {
                    expected: format!("{:?}", col.sql_type),
                    found: format!("{:?}", val),
                });
            }
        }

        Ok(())
    }
}
