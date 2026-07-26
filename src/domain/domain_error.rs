use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    TypeMismatch {
        expected: String,
        found: String,
    },
    InvalidExpression(String),
    EvaluationError(String),

    /// Error saat ekstraksi/konversi `SqlValue` ke tipe Rust asli gagal.
    Conversion {
        expected: &'static str,
        found: &'static str,
    },
}

impl DomainError {
    /// Helper constructor idiomatik untuk error konversi.
    pub fn conversion(expected: &'static str, found: &'static str) -> Self {
        Self::Conversion { expected, found }
    }
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DomainError::TypeMismatch { expected, found } => {
                write!(
                    f,
                    "Tipe tidak cocok: diharapkan '{expected}', ditemukan '{found}'"
                )
            }
            Self::Conversion { expected, found } => {
                write!(
                    f,
                    "Gagal konversi SqlValue: mengharapkan tipe '{expected}', tetapi menemukan tipe '{found}'"
                )
            }
            DomainError::InvalidExpression(msg) => write!(f, "Ekspresi tidak valid: {msg}"),
            DomainError::EvaluationError(msg) => write!(f, "Gagal mengevaluasi: {msg}"),
        }
    }
}

impl std::error::Error for DomainError {}
