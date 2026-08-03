use serde::{Deserialize, Serialize};
use std::{collections::HashSet, ops::Not};

/// Representasi Skema Tipe Data SQL
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SqlType {
    // primitif type
    Int,
    Float,
    Text,
    Bool,
    Bytes,
    // date and time
    Timestamp,
    Date,
    Time,
    /// Custom Enum dengan nama dan varian yang diizinkan
    Enum {
        name: String,
        variants: Vec<String>,
    },
    Custom(String),
}

impl SqlType {
    /// Memvalidasi definisi SqlType, memastikan tidak ada varian Enum yang duplikat
    pub fn validate_enum_variants(&self) -> Result<(), DomainError> {
        if let SqlType::Enum { name, variants } = self {
            let mut seen = HashSet::with_capacity(variants.len());

            for variant in variants {
                // hashset.insert() mengembalikan false jika element sudah ada sebelumnya
                if !seen.insert(variant) {
                    return Err(DomainError::EvaluationError(format!(
                        "Definisi Enum '{name}' tidak valid: varian '{variant}' terdefinisi lebih dari sekali"
                    )));
                }
            }
        }

        Ok(())
    }
}

/// untuk evaluasi ekspresi atau perbandingan kwhere (And Or Not)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SqlBool {
    True,
    False,
    Unknown,
}

// Implementasi std::ops::Not untuk 3VL
impl Not for SqlBool {
    type Output = Self;

    /// Helper logika NOT ala Three-Valued Logic SQL
    fn not(self) -> Self::Output {
        match self {
            SqlBool::True => SqlBool::False,
            SqlBool::False => SqlBool::True,
            SqlBool::Unknown => SqlBool::Unknown,
        }
    }
}

impl SqlBool {
    /// Mengembalikan true HANYA jika nilainya murni `SqlBool::True`
    pub fn is_true(&self) -> bool {
        matches!(self, SqlBool::True)
    }

    /// Helper logika AND ala Three-Valued Logic SQL
    pub fn and(self, other: Self) -> Self {
        match (self, other) {
            (SqlBool::False, _) | (_, SqlBool::False) => SqlBool::False,
            (SqlBool::True, SqlBool::True) => SqlBool::True,
            _ => SqlBool::Unknown,
        }
    }

    /// Helper logika OR ala Three-Valued Logic SQL
    pub fn or(self, other: Self) -> Self {
        match (self, other) {
            (SqlBool::True, _) | (_, SqlBool::True) => SqlBool::True,
            (SqlBool::False, SqlBool::False) => SqlBool::False,
            _ => SqlBool::Unknown,
        }
    }
}

// =============================================================================
// IMPLEMENTASI TRAIT CONVERSION (IDIOMATIC RUST)
// =============================================================================

/// Mengubah `bool` Rust (true/false) menjadi `SqlBool` (True/False)
impl From<bool> for SqlBool {
    fn from(cond: bool) -> Self {
        if cond { SqlBool::True } else { SqlBool::False }
    }
}

use std::convert::TryFrom;

use crate::{DomainError, SqlValue};

// Implementasi untuk Referensi (&SqlValue)
impl TryFrom<&SqlValue> for SqlBool {
    type Error = DomainError;

    /// Conversion strict dari SqlValue ke SqlBool untuk operasi logika (AND/OR/NOT).
    /// Mengembalikan TypeError jika tipe data bukan Bool atau Null.
    fn try_from(value: &SqlValue) -> Result<Self, Self::Error> {
        match value {
            SqlValue::Bool(b) => Ok(SqlBool::from(*b)),
            SqlValue::Null => Ok(SqlBool::Unknown),
            other => Err(DomainError::EvaluationError(format!(
                "Operasi logika membutuhkan tipe BOOLEAN, tetapi mendapatkan {:?}",
                other
            ))),
        }
    }
}

// Implementasi untuk Owned (SqlValue)
impl TryFrom<SqlValue> for SqlBool {
    type Error = DomainError;

    /// Conversion strict dari SqlValue ke SqlBool untuk operasi logika (AND/OR/NOT).
    /// Mengembalikan TypeError jika tipe data bukan Bool atau Null.
    fn try_from(value: SqlValue) -> Result<Self, Self::Error> {
        SqlBool::try_from(&value)
    }
}
