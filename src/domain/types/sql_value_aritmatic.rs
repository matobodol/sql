use ordered_float::OrderedFloat;
use regex::Regex;

use crate::{DomainError, SqlBool, SqlValue};

impl SqlValue {
    // =========================================================================
    // PATTERN MATCHING & LOGICAL HELPERS (3VL)
    // =========================================================================

    /// Helper internal mengekstraksi representasi string untuk operasi LIKE
    fn as_str_for_like(&self) -> Option<&str> {
        match self {
            SqlValue::Text(s) => Some(s.as_str()),
            SqlValue::Enum { value, .. } => Some(value.as_str()),
            SqlValue::Custom { value, .. } => Some(value.as_str()),
            _ => None,
        }
    }

    /// Operator LIKE dengan aturan ANSI SQL 3VL (Mendukung Text, Enum, & Custom)
    pub fn like(&self, pattern: &Self) -> Result<SqlBool, DomainError> {
        if self.is_null() || pattern.is_null() {
            return Ok(SqlBool::Unknown);
        }

        match (self.as_str_for_like(), pattern.as_str_for_like()) {
            (Some(text), Some(pat)) => {
                let mut regex_str = String::from("^");
                for ch in pat.chars() {
                    match ch {
                        '%' => regex_str.push_str(".*"),
                        '_' => regex_str.push('.'),
                        '\\' | '.' | '+' | '*' | '?' | '(' | ')' | '|' | '[' | ']' | '{' | '}'
                        | '^' | '$' => {
                            regex_str.push('\\');
                            regex_str.push(ch);
                        }
                        c => regex_str.push(c),
                    }
                }
                regex_str.push('$');

                let re = Regex::new(&regex_str).map_err(|e| {
                    DomainError::EvaluationError(format!("Pola LIKE '{pat}' tidak valid: {e}"))
                })?;

                Ok(SqlBool::from(re.is_match(text)))
            }
            _ => Err(DomainError::EvaluationError(
                "Operan untuk operator LIKE harus bertipe Text, Enum, atau Custom".into(),
            )),
        }
    }

    // =========================================================================
    // ARITMATIKA (SSOT - Digunakan oleh Evaluator & Aggregate Accumulator)
    // =========================================================================

    /// Operasi Penjumlahan (+)
    pub fn add(&self, other: &Self) -> Result<SqlValue, DomainError> {
        if self.is_null() || other.is_null() {
            return Ok(SqlValue::Null);
        }

        match (self, other) {
            (SqlValue::Int(a), SqlValue::Int(b)) => Ok(SqlValue::Int(a + b)),
            (SqlValue::Float(a), SqlValue::Float(b)) => Ok(SqlValue::Float(a + b)),
            (SqlValue::Int(a), SqlValue::Float(b)) => {
                Ok(SqlValue::Float(OrderedFloat::from(*a as f64) + b))
            }
            (SqlValue::Float(a), SqlValue::Int(b)) => Ok(SqlValue::Float(a + *b as f64)),
            _ => Err(DomainError::EvaluationError(
                "Tipe data tidak valid untuk operasi penjumlahan".into(),
            )),
        }
    }

    /// Operasi Pengurangan (-)
    pub fn sub(&self, other: &Self) -> Result<SqlValue, DomainError> {
        if self.is_null() || other.is_null() {
            return Ok(SqlValue::Null);
        }

        match (self, other) {
            (SqlValue::Int(a), SqlValue::Int(b)) => Ok(SqlValue::Int(a - b)),
            (SqlValue::Float(a), SqlValue::Float(b)) => Ok(SqlValue::Float(a - b)),
            (SqlValue::Int(a), SqlValue::Float(b)) => {
                Ok(SqlValue::Float(OrderedFloat::from(*a as f64) - b))
            }
            (SqlValue::Float(a), SqlValue::Int(b)) => Ok(SqlValue::Float(a - *b as f64)),
            _ => Err(DomainError::EvaluationError(
                "Tipe data tidak valid untuk operasi pengurangan".into(),
            )),
        }
    }

    /// Operasi Perkalian (*)
    pub fn mul(&self, other: &Self) -> Result<SqlValue, DomainError> {
        if self.is_null() || other.is_null() {
            return Ok(SqlValue::Null);
        }

        match (self, other) {
            (SqlValue::Int(a), SqlValue::Int(b)) => Ok(SqlValue::Int(a * b)),
            (SqlValue::Float(a), SqlValue::Float(b)) => Ok(SqlValue::Float(a * b)),
            (SqlValue::Int(a), SqlValue::Float(b)) => {
                Ok(SqlValue::Float(OrderedFloat::from(*a as f64) * b))
            }
            (SqlValue::Float(a), SqlValue::Int(b)) => Ok(SqlValue::Float(a * *b as f64)),
            _ => Err(DomainError::EvaluationError(
                "Tipe data tidak valid meperoleh operasi perkalian".into(),
            )),
        }
    }

    /// Operasi Pembagian (/) dengan proteksi Division by Zero
    pub fn div(&self, other: &Self) -> Result<SqlValue, DomainError> {
        if self.is_null() || other.is_null() {
            return Ok(SqlValue::Null);
        }

        // Cek Division by Zero
        match other {
            SqlValue::Int(0) => {
                return Err(DomainError::EvaluationError(
                    "Pembagian dengan nol (Division by zero)".into(),
                ));
            }
            SqlValue::Float(f) if f.into_inner() == 0.0 => {
                return Err(DomainError::EvaluationError(
                    "Pembagian dengan nol (Division by zero)".into(),
                ));
            }
            _ => {}
        }

        match (self, other) {
            (SqlValue::Int(a), SqlValue::Int(b)) => Ok(SqlValue::Int(a / b)),
            (SqlValue::Float(a), SqlValue::Float(b)) => Ok(SqlValue::Float(a / b)),
            (SqlValue::Int(a), SqlValue::Float(b)) => {
                Ok(SqlValue::Float(OrderedFloat::from(*a as f64) / b))
            }
            // (SqlValue::Float(a), SqlValue::Float(b)) => Ok(SqlValue::Float(a / b)),
            _ => Err(DomainError::EvaluationError(
                "Tipe data tidak valid untuk operasi pembagian".into(),
            )),
        }
    }
}
