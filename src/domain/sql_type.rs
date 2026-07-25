use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

// =============================================================================
// 1. DEFINISI ENUM TYPE & VALUE
// =============================================================================

/// Representasi Skema Tipe Data SQL
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SqlType {
    Int,
    Float,
    Text,
    Bool,
    Timestamp,
    Bytes,
    /// Custom Enum dengan nama dan varian yang diizinkan
    Enum {
        name: String,
        variants: Vec<String>,
    },
    Custom(String),
}

/// Representasi Nilai Data SQL di Runtime
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SqlValue {
    Int(i64),
    Float(f64),
    Text(String),
    Bool(bool),
    /// Satu-satunya tipe waktu, selalu disimpan dalam UTC (Single Source of Truth)
    Timestamp(DateTime<Utc>),
    Bytes(Vec<u8>),
    Null,
}

impl SqlValue {
    /// Helper untuk mendapatkan waktu saat ini dalam UTC
    pub fn now() -> Self {
        SqlValue::Timestamp(Utc::now())
    }

    /// Konversi ke String waktu lokal OS (Hanya untuk Tampilan/UI)
    pub fn to_local_string(&self) -> String {
        match self {
            SqlValue::Timestamp(utc_dt) => {
                let local_dt: DateTime<Local> = DateTime::from(*utc_dt);
                local_dt.format("%A, %d %B %Y %H:%M:%S (%Z)").to_string()
            }
            SqlValue::Text(s) => s.clone(),
            SqlValue::Int(n) => n.to_string(),
            SqlValue::Float(f) => f.to_string(),
            SqlValue::Bool(b) => b.to_string(),
            SqlValue::Bytes(b) => format!("<{} bytes>", b.len()),
            SqlValue::Null => "NULL".to_string(),
        }
    }

    /// Validasi tipe data sesuai skema SqlType
    pub fn matches_type(&self, sql_type: &SqlType) -> bool {
        match (self, sql_type) {
            (SqlValue::Null, _) => true,
            (SqlValue::Int(_), SqlType::Int) => true,
            (SqlValue::Float(_), SqlType::Float) => true,
            (SqlValue::Text(_), SqlType::Text) => true,
            (SqlValue::Bool(_), SqlType::Bool) => true,
            (SqlValue::Timestamp(_), SqlType::Timestamp) => true,
            (SqlValue::Bytes(_), SqlType::Bytes) => true,
            (SqlValue::Text(val), SqlType::Enum { variants, .. }) => variants.contains(val),
            (SqlValue::Text(_), SqlType::Custom(_)) => true,
            _ => false,
        }
    }
}

// =============================================================================
// 2. ERROR HANDLING UNTUK KONVERSI TRYFROM
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlValueConversionError {
    pub expected: &'static str,
    pub found: &'static str,
}

impl SqlValueConversionError {
    pub fn new(expected: &'static str, found: &'static str) -> Self {
        Self { expected, found }
    }
}

impl fmt::Display for SqlValueConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Gagal konversi SqlValue: mengharapkan tipe '{}', tetapi menemukan tipe '{}'",
            self.expected, self.found
        )
    }
}

impl std::error::Error for SqlValueConversionError {}

// =============================================================================
// 3. IMPLEMENTASI `From` (Mengubah Tipe Rust -> SqlValue)
// =============================================================================

impl From<i64> for SqlValue {
    fn from(v: i64) -> Self {
        SqlValue::Int(v)
    }
}
impl From<i32> for SqlValue {
    fn from(v: i32) -> Self {
        SqlValue::Int(v as i64)
    }
}
impl From<usize> for SqlValue {
    fn from(v: usize) -> Self {
        SqlValue::Int(v as i64)
    }
}
impl From<f64> for SqlValue {
    fn from(v: f64) -> Self {
        SqlValue::Float(v)
    }
}
impl From<f32> for SqlValue {
    fn from(v: f32) -> Self {
        SqlValue::Float(v as f64)
    }
}
impl From<String> for SqlValue {
    fn from(v: String) -> Self {
        SqlValue::Text(v)
    }
}
impl From<&str> for SqlValue {
    fn from(v: &str) -> Self {
        SqlValue::Text(v.to_string())
    }
}
impl From<bool> for SqlValue {
    fn from(v: bool) -> Self {
        SqlValue::Bool(v)
    }
}
impl From<DateTime<Utc>> for SqlValue {
    fn from(v: DateTime<Utc>) -> Self {
        SqlValue::Timestamp(v)
    }
}
impl From<Vec<u8>> for SqlValue {
    fn from(v: Vec<u8>) -> Self {
        SqlValue::Bytes(v)
    }
}
impl From<&[u8]> for SqlValue {
    fn from(v: &[u8]) -> Self {
        SqlValue::Bytes(v.to_vec())
    }
}

// Support otomatis untuk Option<T> (Ganti Option::None menjadi SqlValue::Null)
impl<T> From<Option<T>> for SqlValue
where
    SqlValue: From<T>,
{
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(v) => SqlValue::from(v),
            None => SqlValue::Null,
        }
    }
}

// =============================================================================
// 4. IMPLEMENTASI `TryFrom` (Mengekstrak SqlValue -> Tipe Rust Asli)
// =============================================================================

// --- Integer ---
impl TryFrom<SqlValue> for i64 {
    type Error = SqlValueConversionError;

    fn try_from(val: SqlValue) -> Result<Self, Self::Error> {
        match val {
            SqlValue::Int(n) => Ok(n),
            other => Err(SqlValueConversionError::new(
                "i64",
                get_variant_name(&other),
            )),
        }
    }
}

impl TryFrom<SqlValue> for i32 {
    type Error = SqlValueConversionError;

    fn try_from(val: SqlValue) -> Result<Self, Self::Error> {
        match val {
            SqlValue::Int(n) => n
                .try_into()
                .map_err(|_| SqlValueConversionError::new("i32 (out of bounds)", "i64")),
            other => Err(SqlValueConversionError::new(
                "i32",
                get_variant_name(&other),
            )),
        }
    }
}

// --- Float ---
impl TryFrom<SqlValue> for f64 {
    type Error = SqlValueConversionError;

    fn try_from(val: SqlValue) -> Result<Self, Self::Error> {
        match val {
            SqlValue::Float(f) => Ok(f),
            other => Err(SqlValueConversionError::new(
                "f64",
                get_variant_name(&other),
            )),
        }
    }
}

// --- Text / String ---
impl TryFrom<SqlValue> for String {
    type Error = SqlValueConversionError;

    fn try_from(val: SqlValue) -> Result<Self, Self::Error> {
        match val {
            SqlValue::Text(s) => Ok(s),
            other => Err(SqlValueConversionError::new(
                "String",
                get_variant_name(&other),
            )),
        }
    }
}

// --- Boolean ---
impl TryFrom<SqlValue> for bool {
    type Error = SqlValueConversionError;

    fn try_from(val: SqlValue) -> Result<Self, Self::Error> {
        match val {
            SqlValue::Bool(b) => Ok(b),
            other => Err(SqlValueConversionError::new(
                "bool",
                get_variant_name(&other),
            )),
        }
    }
}

// --- Timestamp ---
impl TryFrom<SqlValue> for DateTime<Utc> {
    type Error = SqlValueConversionError;

    fn try_from(val: SqlValue) -> Result<Self, Self::Error> {
        match val {
            SqlValue::Timestamp(dt) => Ok(dt),
            other => Err(SqlValueConversionError::new(
                "DateTime<Utc>",
                get_variant_name(&other),
            )),
        }
    }
}

// --- Bytes ---
impl TryFrom<SqlValue> for Vec<u8> {
    type Error = SqlValueConversionError;

    fn try_from(val: SqlValue) -> Result<Self, Self::Error> {
        match val {
            SqlValue::Bytes(b) => Ok(b),
            other => Err(SqlValueConversionError::new(
                "Vec<u8>",
                get_variant_name(&other),
            )),
        }
    }
}

// --- Support Option<T> untuk kolom yang BISA NULL (Nullable Column) ---
impl<T> TryFrom<SqlValue> for Option<T>
where
    T: TryFrom<SqlValue, Error = SqlValueConversionError>,
{
    type Error = SqlValueConversionError;

    fn try_from(val: SqlValue) -> Result<Self, Self::Error> {
        match val {
            SqlValue::Null => Ok(None),
            other => T::try_from(other).map(Some),
        }
    }
}

/// Helper function internal untuk menghasilkan nama varian saat error
fn get_variant_name(val: &SqlValue) -> &'static str {
    match val {
        SqlValue::Int(_) => "Int",
        SqlValue::Float(_) => "Float",
        SqlValue::Text(_) => "Text",
        SqlValue::Bool(_) => "Bool",
        SqlValue::Timestamp(_) => "Timestamp",
        SqlValue::Bytes(_) => "Bytes",
        SqlValue::Null => "Null",
    }
}
