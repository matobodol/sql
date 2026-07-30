use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    #[error("Tipe tidak cocok: diharapkan '{expected}', ditemukan '{found}'")]
    TypeMismatch { expected: String, found: String },

    #[error("Ekspresi tidak valid: {0}")]
    InvalidExpression(String),

    #[error("Gagal mengevaluasi: {0}")]
    EvaluationError(String),

    /// Error saat ekstraksi/konversi `SqlValue` ke tipe Rust asli gagal.
    #[error(
        "Gagal konversi SqlValue: mengharapkan tipe '{expected}', tetapi menemukan tipe '{found}'"
    )]
    Conversion {
        expected: &'static str,
        found: &'static str,
    },

    #[error("Tabel '{0}' tidak ditemukan")]
    TableNotFound(String),

    #[error("Tabel '{0}' sudah ada")]
    TableAlreadyExists(String),
}

impl DomainError {
    /// Helper constructor idiomatik untuk error konversi.
    pub fn conversion(expected: &'static str, found: &'static str) -> Self {
        Self::Conversion { expected, found }
    }
}
