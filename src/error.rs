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

    // --- Varian Tambahan untuk User & Database Management ---
    #[error("User '{0}' tidak ditemukan")]
    UserNotFound(Arc<str>),

    #[error("User '{0}' sudah ada")]
    UserAlreadyExists(Arc<str>),

    #[error("Password tidak valid untuk user '{0}'")]
    UserPasswordInvalid(Arc<str>),

    #[error("Database '{0}' tidak ditemukan")]
    DatabaseNotFound(Arc<str>),

    #[error("Database '{0}' sudah ada")]
    DatabaseAlreadyExists(Arc<str>),

    #[error("Tidak ada database aktif")]
    NoActiveDatabase,

    #[error("{0}")]
    Catalog(Arc<str>),
}

impl DomainError {
    #[inline]
    pub fn conversion(expected: &'static str, found: &'static str) -> Self {
        Self::Conversion { expected, found }
    }

    #[inline]
    pub fn invalid_expr(msg: impl Into<Arc<str>>) -> Self {
        Self::InvalidExpression(msg.into())
    }

    #[inline]
    pub fn eval_error(msg: impl Into<Arc<str>>) -> Self {
        Self::EvaluationError(msg.into())
    }

    #[inline]
    pub fn exec_error(msg: impl Into<Arc<str>>) -> Self {
        Self::ExecutionError(msg.into())
    }

    #[inline]
    pub fn catalog(msg: impl Into<Arc<str>>) -> Self {
        Self::Catalog(msg.into())
    }
}
