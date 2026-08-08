use ordered_float::OrderedFloat;
use regex::Regex;

use crate::{DomainError, SqlBool, ValueType};

impl ValueType {
    /// Helper internal zero-copy mengekstraksi string reference
    #[inline]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            ValueType::Text(s) => Some(s),
            ValueType::Enum { value, .. } => Some(value),
            ValueType::Custom { value, .. } => Some(value),
            _ => None,
        }
    }

    /// Evaluasi LIKE berbasis Pre-compiled Regex (menerima `&Regex` terkompilasi dari Evaluator)
    pub fn like_compiled(&self, re: &Regex) -> Result<SqlBool, DomainError> {
        if self.is_null() {
            return Ok(SqlBool::Unknown);
        }

        if let Some(text) = self.as_str() {
            Ok(SqlBool::from(re.is_match(text)))
        } else {
            Err(DomainError::eval_error(
                "Operan untuk operator LIKE harus bertipe Text, Enum, atau Custom",
            ))
        }
    }

    /// Fallback Evaluasi LIKE standar
    pub fn like(&self, pattern: &Self) -> Result<SqlBool, DomainError> {
        if self.is_null() || pattern.is_null() {
            return Ok(SqlBool::Unknown);
        }

        match (self.as_str(), pattern.as_str()) {
            (Some(text), Some(pat)) => {
                let regex_pattern = parse_sql_like_pattern(pat);
                let re = Regex::new(&regex_pattern).map_err(|e| {
                    DomainError::eval_error(format!("Pola LIKE '{pat}' tidak valid: {e}"))
                })?;

                Ok(SqlBool::from(re.is_match(text)))
            }
            _ => Err(DomainError::eval_error(
                "Operan untuk operator LIKE harus bertipe Text, Enum, atau Custom",
            )),
        }
    }

    // =========================================================================
    // ARITMATIKA (Zero-Copy Referensi)
    // =========================================================================

    #[inline]
    pub fn add(&self, other: &Self) -> Result<ValueType, DomainError> {
        if self.is_null() || other.is_null() {
            return Ok(ValueType::Null);
        }

        match (self, other) {
            (ValueType::Int(a), ValueType::Int(b)) => Ok(ValueType::Int(a + b)),
            (ValueType::Float(a), ValueType::Float(b)) => Ok(ValueType::Float(a + b)),
            (ValueType::Int(a), ValueType::Float(b)) => {
                Ok(ValueType::Float(OrderedFloat(*a as f64) + b))
            }
            (ValueType::Float(a), ValueType::Int(b)) => Ok(ValueType::Float(a + *b as f64)),
            _ => Err(DomainError::eval_error(
                "Tipe data tidak valid untuk operasi penjumlahan",
            )),
        }
    }

    #[inline]
    pub fn sub(&self, other: &Self) -> Result<ValueType, DomainError> {
        if self.is_null() || other.is_null() {
            return Ok(ValueType::Null);
        }

        match (self, other) {
            (ValueType::Int(a), ValueType::Int(b)) => Ok(ValueType::Int(a - b)),
            (ValueType::Float(a), ValueType::Float(b)) => Ok(ValueType::Float(a - b)),
            (ValueType::Int(a), ValueType::Float(b)) => {
                Ok(ValueType::Float(OrderedFloat(*a as f64) - b))
            }
            (ValueType::Float(a), ValueType::Int(b)) => Ok(ValueType::Float(a - *b as f64)),
            _ => Err(DomainError::eval_error(
                "Tipe data tidak valid untuk operasi pengurangan",
            )),
        }
    }

    #[inline]
    pub fn mul(&self, other: &Self) -> Result<ValueType, DomainError> {
        if self.is_null() || other.is_null() {
            return Ok(ValueType::Null);
        }

        match (self, other) {
            (ValueType::Int(a), ValueType::Int(b)) => Ok(ValueType::Int(a * b)),
            (ValueType::Float(a), ValueType::Float(b)) => Ok(ValueType::Float(a * b)),
            (ValueType::Int(a), ValueType::Float(b)) => {
                Ok(ValueType::Float(OrderedFloat(*a as f64) * b))
            }
            (ValueType::Float(a), ValueType::Int(b)) => Ok(ValueType::Float(a * *b as f64)),
            _ => Err(DomainError::eval_error(
                "Tipe data tidak valid untuk operasi perkalian",
            )),
        }
    }

    #[inline]
    pub fn div(&self, other: &Self) -> Result<ValueType, DomainError> {
        if self.is_null() || other.is_null() {
            return Ok(ValueType::Null);
        }

        match other {
            ValueType::Int(0) => {
                return Err(DomainError::eval_error(
                    "Pembagian dengan nol (Division by zero)",
                ));
            }
            ValueType::Float(f) if f.into_inner() == 0.0 => {
                return Err(DomainError::eval_error(
                    "Pembagian dengan nol (Division by zero)",
                ));
            }
            _ => {}
        }

        match (self, other) {
            (ValueType::Int(a), ValueType::Int(b)) => Ok(ValueType::Int(a / b)),
            (ValueType::Float(a), ValueType::Float(b)) => Ok(ValueType::Float(a / b)),
            (ValueType::Int(a), ValueType::Float(b)) => {
                Ok(ValueType::Float(OrderedFloat(*a as f64) / b))
            }
            (ValueType::Float(a), ValueType::Int(b)) => Ok(ValueType::Float(a / *b as f64)),
            _ => Err(DomainError::eval_error(
                "Tipe data tidak valid untuk operasi pembagian",
            )),
        }
    }
}

/// Helper konversi SQL Pattern ke RegEx Pattern
pub fn parse_sql_like_pattern(pat: &str) -> String {
    let mut regex_str = String::with_capacity(pat.len() + 2);
    regex_str.push('^');
    for ch in pat.chars() {
        match ch {
            '%' => regex_str.push_str(".*"),
            '_' => regex_str.push('.'),
            '\\' | '.' | '+' | '*' | '?' | '(' | ')' | '|' | '[' | ']' | '{' | '}' | '^' | '$' => {
                regex_str.push('\\');
                regex_str.push(ch);
            }
            c => regex_str.push(c),
        }
    }
    regex_str.push('$');
    regex_str
}
