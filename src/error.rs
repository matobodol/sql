use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    #[error("Tipe tidak cocok: diharapkan '{expected}', ditemukan '{found}'")]
    TypeMismatch { expected: Arc<str>, found: Arc<str> },

    #[error("Ekspresi tidak valid: {0}")]
    InvalidExpression(Arc<str>),

    #[error("Gagal mengevaluasi: {0}")]
    EvaluationError(Arc<str>),

    #[error("Gagal mengeksekusi: {0}")]
    ExecutionError(Arc<str>),

    /// Error saat ekstraksi/konversi `SqlValue` ke tipe Rust asli gagal.
    #[error(
        "Gagal konversi SqlValue: mengharapkan tipe '{expected}', tetapi menemukan tipe '{found}'"
    )]
    Conversion {
        expected: &'static str,
        found: &'static str,
    },

    #[error("Kolom '{0}' tidak ditemukan")]
    ColumnNotFound(Arc<str>),

    #[error("Kolom '{0}' sudah ada")]
    ColumnAlreadyExists(Arc<str>),

    #[error("Tabel '{0}' tidak ditemukan")]
    TableNotFound(Arc<str>),

    #[error("Tabel '{0}' sudah ada")]
    TableAlreadyExists(Arc<str>),
}

impl DomainError {
    /// Helper constructor idiomatik untuk error konversi.
    #[inline]
    pub fn conversion(expected: &'static str, found: &'static str) -> Self {
        Self::Conversion { expected, found }
    }

    /// Helper constructor fleksibel untuk InvalidExpression
    #[inline]
    pub fn invalid_expr(msg: impl Into<Arc<str>>) -> Self {
        Self::InvalidExpression(msg.into())
    }

    /// Helper constructor fleksibel untuk EvaluationError
    #[inline]
    pub fn eval_error(msg: impl Into<Arc<str>>) -> Self {
        Self::EvaluationError(msg.into())
    }

    /// Helper constructor fleksibel untuk ExecutionError
    #[inline]
    pub fn exec_error(msg: impl Into<Arc<str>>) -> Self {
        Self::ExecutionError(msg.into())
    }
}
