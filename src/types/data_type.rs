use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::ops::Not;

use crate::{DomainError, ValueType};

/// Representasi Skema Tipe Data SQL
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataType {
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

impl DataType {
    /// Memvalidasi definisi SqlType, memastikan tidak ada varian Enum yang duplikat
    pub fn validate_enum_variants(&self) -> Result<(), DomainError> {
        if let DataType::Enum { name, variants } = self {
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
pub enum Bool3VL {
    True,
    False,
    Unknown,
}

impl Not for Bool3VL {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        match self {
            Bool3VL::True => Bool3VL::False,
            Bool3VL::False => Bool3VL::True,
            Bool3VL::Unknown => Bool3VL::Unknown,
        }
    }
}

impl Bool3VL {
    #[inline]
    pub fn is_true(&self) -> bool {
        matches!(self, Bool3VL::True)
    }

    #[inline]
    pub fn and(self, other: Self) -> Self {
        match (self, other) {
            (Bool3VL::False, _) | (_, Bool3VL::False) => Bool3VL::False,
            (Bool3VL::True, Bool3VL::True) => Bool3VL::True,
            _ => Bool3VL::Unknown,
        }
    }

    #[inline]
    pub fn or(self, other: Self) -> Self {
        match (self, other) {
            (Bool3VL::True, _) | (_, Bool3VL::True) => Bool3VL::True,
            (Bool3VL::False, Bool3VL::False) => Bool3VL::False,
            _ => Bool3VL::Unknown,
        }
    }
}

// --- CONVERSION IMPLEMENTATIONS ---

impl From<bool> for Bool3VL {
    #[inline]
    fn from(cond: bool) -> Self {
        if cond { Bool3VL::True } else { Bool3VL::False }
    }
}

impl TryFrom<&ValueType> for Bool3VL {
    type Error = DomainError;

    fn try_from(value: &ValueType) -> Result<Self, Self::Error> {
        match value {
            ValueType::Bool(b) => Ok(Bool3VL::from(*b)),
            ValueType::Null => Ok(Bool3VL::Unknown),
            other => Err(DomainError::eval_error(format!(
                "Operasi logika membutuhkan tipe BOOLEAN, tetapi mendapatkan {:?}",
                other
            ))),
        }
    }
}

impl TryFrom<ValueType> for Bool3VL {
    type Error = DomainError;

    #[inline]
    fn try_from(value: ValueType) -> Result<Self, Self::Error> {
        Bool3VL::try_from(&value)
    }
}
