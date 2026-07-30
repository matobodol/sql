use serde::{Deserialize, Serialize};
use std::ops::Not;

use crate::SqlValue;

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

    /// Conversion ke bool biasa untuk clause WHERE (Hanya TRUE yang lolos filter)
    pub fn is_true(self) -> bool {
        matches!(self, SqlBool::True)
    }

    /// Konversi 3VL ke SqlValue runtime (True -> Bool(true), Unknown -> Null)
    pub fn into_sql_value(self) -> SqlValue {
        self.into()
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

/// Mengubah `SqlValue` (true/false/null) menjadi `SqlBool` (True/False/Unknown)
impl From<&SqlValue> for SqlBool {
    fn from(val: &SqlValue) -> Self {
        match val {
            SqlValue::Bool(b) => (*b).into(),
            SqlValue::Null => SqlBool::Unknown,
            // Semua tipe non-boolean dianggap Unknown di konteks logika 3VL
            _ => SqlBool::Unknown,
        }
    }
}

// Support konversi dari owned SqlValue juga
impl From<SqlValue> for SqlBool {
    fn from(val: SqlValue) -> Self {
        SqlBool::from(&val)
    }
}
