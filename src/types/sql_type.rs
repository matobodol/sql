use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::ops::Not;

use crate::{DomainError, SqlValue};

/// Representasi Skema Tipe Data SQL
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SqlType {
    Int,
    Float,
    Text,
    Bool,
    Bytes,
    Timestamp,
    Date,
    Time,
    Enum { name: String, variants: Vec<String> },
    Custom(String),
}

impl SqlType {
    /// Memvalidasi definisi SqlType, memastikan tidak ada varian Enum yang duplikat
    pub fn validate_enum_variants(&self) -> Result<(), DomainError> {
        if let SqlType::Enum { name, variants } = self {
            let mut seen = HashSet::with_capacity(variants.len());

            for variant in variants {
                if !seen.insert(variant) {
                    return Err(DomainError::eval_error(format!(
                        "Definisi Enum '{name}' tidak valid: varian '{variant}' terdefinisi lebih dari sekali"
                    )));
                }
            }
        }

        Ok(())
    }
}

/// Evaluasi logika Three-Valued Logic (3VL)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SqlBool {
    True,
    False,
    Unknown,
}

impl Not for SqlBool {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        match self {
            SqlBool::True => SqlBool::False,
            SqlBool::False => SqlBool::True,
            SqlBool::Unknown => SqlBool::Unknown,
        }
    }
}

impl SqlBool {
    #[inline]
    pub fn is_true(&self) -> bool {
        matches!(self, SqlBool::True)
    }

    #[inline]
    pub fn and(self, other: Self) -> Self {
        match (self, other) {
            (SqlBool::False, _) | (_, SqlBool::False) => SqlBool::False,
            (SqlBool::True, SqlBool::True) => SqlBool::True,
            _ => SqlBool::Unknown,
        }
    }

    #[inline]
    pub fn or(self, other: Self) -> Self {
        match (self, other) {
            (SqlBool::True, _) | (_, SqlBool::True) => SqlBool::True,
            (SqlBool::False, SqlBool::False) => SqlBool::False,
            _ => SqlBool::Unknown,
        }
    }
}

// --- CONVERSION IMPLEMENTATIONS ---

impl From<bool> for SqlBool {
    #[inline]
    fn from(cond: bool) -> Self {
        if cond { SqlBool::True } else { SqlBool::False }
    }
}

impl TryFrom<&SqlValue> for SqlBool {
    type Error = DomainError;

    fn try_from(value: &SqlValue) -> Result<Self, Self::Error> {
        match value {
            SqlValue::Bool(b) => Ok(SqlBool::from(*b)),
            SqlValue::Null => Ok(SqlBool::Unknown),
            other => Err(DomainError::eval_error(format!(
                "Operasi logika membutuhkan tipe BOOLEAN, tetapi mendapatkan {:?}",
                other
            ))),
        }
    }
}

impl TryFrom<SqlValue> for SqlBool {
    type Error = DomainError;

    #[inline]
    fn try_from(value: SqlValue) -> Result<Self, Self::Error> {
        SqlBool::try_from(&value)
    }
}
