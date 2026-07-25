use chrono::{DateTime, Local, NaiveDate, NaiveTime, Utc};
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
    Date,
    Time,
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
    /// Tipe waktu, selalu disimpan dalam UTC (Single Source of Truth)
    Timestamp(DateTime<Utc>),
    Date(NaiveDate),
    Time(NaiveTime),
    Bytes(Vec<u8>),
    Null,
}

impl SqlValue {
    // --- CONSTRUCTOR HELPER (AUTO DATE/TIME DARI CHRONO UTCTIMESTAMP) ---

    /// Mengambil Timestamp saat ini dalam UTC
    pub fn now() -> Self {
        SqlValue::Timestamp(Utc::now())
    }

    /// Auto-extract komponen DATE (Lokal) langsung dari DateTime<Utc>
    pub fn date_from_datetime(dt: DateTime<Utc>) -> Self {
        let local_dt: DateTime<Local> = DateTime::from(dt);
        SqlValue::Date(local_dt.date_naive())
    }

    /// Auto-extract komponen TIME (Lokal) langsung dari DateTime<Utc>
    pub fn time_from_datetime(dt: DateTime<Utc>) -> Self {
        let local_dt: DateTime<Local> = DateTime::from(dt);
        SqlValue::Time(local_dt.time())
    }

    /// Helper instan: Ambil tanggal lokal HARI INI
    pub fn today() -> Self {
        Self::date_from_datetime(Utc::now())
    }

    /// Helper instan: Ambil jam lokal SAAT INI
    pub fn current_time() -> Self {
        Self::time_from_datetime(Utc::now())
    }

    // --- PARSER HELPER FOR MANUAL INPUT ---

    /// Parse manual dari String ke Date ("YYYY-MM-DD")
    pub fn parse_date(input: &str) -> Result<Self, String> {
        NaiveDate::parse_from_str(input, "%Y-%m-%d")
            .map(SqlValue::Date)
            .map_err(|e| format!("Format tanggal salah (Gunakan YYYY-MM-DD): {e}"))
    }

    /// Parse manual dari String ke Time ("HH:MM:SS" atau "HH:MM")
    pub fn parse_time(input: &str) -> Result<Self, String> {
        if let Ok(t) = NaiveTime::parse_from_str(input, "%H:%M:%S") {
            return Ok(SqlValue::Time(t));
        }
        NaiveTime::parse_from_str(input, "%H:%M")
            .map(SqlValue::Time)
            .map_err(|e| format!("Format waktu salah (Gunakan HH:MM:SS atau HH:MM): {e}"))
    }

    // --- VALIDASI TIPE ---
    pub fn matches_type(&self, sql_type: &SqlType) -> bool {
        match (self, sql_type) {
            (SqlValue::Null, _) => true,
            (SqlValue::Int(_), SqlType::Int) => true,
            (SqlValue::Float(_), SqlType::Float) => true,
            (SqlValue::Text(_), SqlType::Text) => true,
            (SqlValue::Bool(_), SqlType::Bool) => true,
            (SqlValue::Timestamp(_), SqlType::Timestamp) => true,
            (SqlValue::Date(_), SqlType::Date) => true,
            (SqlValue::Time(_), SqlType::Time) => true,
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
impl From<NaiveDate> for SqlValue {
    fn from(v: NaiveDate) -> Self {
        SqlValue::Date(v)
    }
}
impl From<NaiveTime> for SqlValue {
    fn from(v: NaiveTime) -> Self {
        SqlValue::Time(v)
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

// --- Date ---
impl TryFrom<SqlValue> for NaiveDate {
    type Error = SqlValueConversionError;

    fn try_from(val: SqlValue) -> Result<Self, Self::Error> {
        match val {
            SqlValue::Date(d) => Ok(d),
            other => Err(SqlValueConversionError::new(
                "NaiveDate",
                get_variant_name(&other),
            )),
        }
    }
}

// --- Time ---
impl TryFrom<SqlValue> for NaiveTime {
    type Error = SqlValueConversionError;

    fn try_from(val: SqlValue) -> Result<Self, Self::Error> {
        match val {
            SqlValue::Time(t) => Ok(t),
            other => Err(SqlValueConversionError::new(
                "NaiveTime",
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
        SqlValue::Date(_) => "Date",
        SqlValue::Time(_) => "Time",
        SqlValue::Bytes(_) => "Bytes",
        SqlValue::Null => "Null",
    }
}

// =============================================================================
// 5. IMPLEMENTASI EKSTRAKSI UTC UNTUK CLIENT
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EkstrakTimeStamp {
    /// Waktu mentah dalam UTC
    pub utc: DateTime<Utc>,
    /// Nama / Abbrev Zona Waktu Lokal perangkat (misal: "WIB", "WITA", "JST", "BST")
    pub zona: String,
    /// Component Tanggal Lokal (YYYY-MM-DD)
    pub date: NaiveDate,
    /// Component Waktu Lokal (HH:MM:SS)
    pub time: NaiveTime,
}

impl EkstrakTimeStamp {
    /// Membuat ekstraksi timestamp berdasarkan Zona Waktu Lokal perangkat (Client)
    pub fn from_utc_local(utc_dt: DateTime<Utc>) -> Self {
        // 1. Konversi dari Utc ke Local time perangkat saat ini
        let local_dt: DateTime<Local> = DateTime::from(utc_dt);

        Self {
            utc: utc_dt,
            // Mengekstrak singkatan nama zona waktu (misal: "WIB", "EST", dll)
            zona: local_dt.format("%Z").to_string(),
            // Mengekstrak komponen Tanggal Murni (Local)
            date: local_dt.date_naive(),
            // Mengekstrak komponen Jam Murni (Local)
            time: local_dt.time(),
        }
    }

    /// String terformat untuk tampilan tanggal (YYYY-MM-DD)
    pub fn formatted_date(&self) -> String {
        self.date.format("%Y-%m-%d").to_string()
    }

    /// String terformat untuk tampilan waktu (HH:MM:SS)
    pub fn formatted_time(&self) -> String {
        self.time.format("%H:%M:%S").to_string()
    }

    /// String lengkap untuk Struk / Resi Bank
    pub fn to_receipt_string(&self) -> String {
        format!(
            "{} {} {}",
            self.formatted_date(),
            self.formatted_time(),
            self.zona
        )
    }
}
