use chrono::{DateTime, Local, NaiveDate, NaiveTime, Utc};
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};

use crate::{DomainError, SqlBool, SqlType};

/// Representasi Nilai Data SQL di Runtime
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SqlValue {
    /// representasi tidak ada nilai
    Null,
    /// bilangan bulat i64
    Int(i64),
    /// bilangan pecahan f64
    Float(OrderedFloat<f64>),
    /// Sql String
    Text(String),
    /// Sql boolean
    Bool(bool),

    /// type file atau media
    Bytes(Vec<u8>),

    /// timestamp, selalu disimpan dalam UTC (Single Source of Truth)
    Timestamp(DateTime<Utc>),
    Date(NaiveDate),
    Time(NaiveTime),

    // UDT User-Defined Tipe atau Domain Type.
    /// Representasi runtime nilai Enum: (Nama Type Enum, Value Variant)
    /// Contoh: SqlValue::Enum { type_name: "status".into(), value: "ACTIVE".into() }
    Enum {
        type_name: String,
        value: String,
    },

    /// Representasi runtime tipe kustom/domain khusus: (Nama Type Custom, Value Raw)
    Custom {
        type_name: String,
        value: String,
    },
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
    pub fn parse_date(input: &str) -> Result<Self, DomainError> {
        NaiveDate::parse_from_str(input, "%Y-%m-%d")
            .map(SqlValue::Date)
            .map_err(|e| {
                DomainError::InvalidExpression(format!(
                    "Format tanggal '{input}' salah (Gunakan YYYY-MM-DD): {e}"
                ))
            })
    }

    /// Parse manual dari String ke Time ("HH:MM:SS" atau "HH:MM")
    pub fn parse_time(input: &str) -> Result<Self, DomainError> {
        if let Ok(t) = NaiveTime::parse_from_str(input, "%H:%M:%S") {
            return Ok(SqlValue::Time(t));
        }

        NaiveTime::parse_from_str(input, "%H:%M")
            .map(SqlValue::Time)
            .map_err(|e| {
                DomainError::InvalidExpression(format!(
                    "Format waktu '{input}' salah (Gunakan HH:MM:SS atau HH:MM): {e}"
                ))
            })
    }

    // --- VALIDASI TIPE ---
    /// validasi type antara value dan schema mengembalikan boolean.
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

            // Validasi Enum: defined/name cocok DAN value ada di daftar variants
            (SqlValue::Enum { type_name, value }, SqlType::Enum { name, variants }) => {
                type_name == name && variants.contains(value)
            }

            // Validasi Custom: type_name harus cocok
            (SqlValue::Custom { type_name, .. }, SqlType::Custom(expected_type)) => {
                type_name == expected_type
            }

            // Casting darurat/kompatibilitas jika dari Text
            // (SqlValue::Text(val), SqlType::Enum { variants, .. }) => variants.contains(val),
            // (SqlValue::Text(_), SqlType::Custom(_)) => true,
            _ => false,
        }
    }
}

// =============================================================================
// 3. IMPLEMENTASI `From` (Mengubah Tipe Rust -> SqlValue)
// =============================================================================

/// Mengubah `SqlBool` (3VL) menjadi `SqlValue` runtime
impl From<&SqlBool> for SqlValue {
    fn from(sb: &SqlBool) -> Self {
        match sb {
            SqlBool::True => SqlValue::Bool(true),
            SqlBool::False => SqlValue::Bool(false),
            SqlBool::Unknown => SqlValue::Null,
        }
    }
}

impl From<SqlBool> for SqlValue {
    fn from(sb: SqlBool) -> Self {
        SqlValue::from(&sb)
    }
}

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
        SqlValue::Float(OrderedFloat(v))
    }
}
impl From<f32> for SqlValue {
    fn from(v: f32) -> Self {
        SqlValue::Float(OrderedFloat(v as f64))
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
    type Error = DomainError;

    fn try_from(val: SqlValue) -> Result<Self, Self::Error> {
        match val {
            SqlValue::Int(n) => Ok(n),
            other => Err(DomainError::conversion("i64", get_variant_name(&other))),
        }
    }
}

impl TryFrom<SqlValue> for i32 {
    type Error = DomainError;

    fn try_from(val: SqlValue) -> Result<Self, Self::Error> {
        match val {
            SqlValue::Int(n) => n
                .try_into()
                .map_err(|_| DomainError::conversion("i32 (out of bounds)", "i64")),
            other => Err(DomainError::conversion("i32", get_variant_name(&other))),
        }
    }
}

// --- Float ---
impl TryFrom<SqlValue> for f64 {
    type Error = DomainError;

    fn try_from(val: SqlValue) -> Result<Self, Self::Error> {
        match val {
            SqlValue::Float(f) => Ok(f.into_inner()),
            other => Err(DomainError::conversion("f64", get_variant_name(&other))),
        }
    }
}

// --- Text / String ---
impl TryFrom<SqlValue> for String {
    type Error = DomainError;

    fn try_from(val: SqlValue) -> Result<Self, Self::Error> {
        match val {
            SqlValue::Text(s) => Ok(s),
            other => Err(DomainError::conversion("String", get_variant_name(&other))),
        }
    }
}

// --- Boolean ---
impl TryFrom<SqlValue> for bool {
    type Error = DomainError;

    fn try_from(val: SqlValue) -> Result<Self, Self::Error> {
        match val {
            SqlValue::Bool(b) => Ok(b),
            other => Err(DomainError::conversion("bool", get_variant_name(&other))),
        }
    }
}

// --- Timestamp ---
impl TryFrom<SqlValue> for DateTime<Utc> {
    type Error = DomainError;

    fn try_from(val: SqlValue) -> Result<Self, Self::Error> {
        match val {
            SqlValue::Timestamp(dt) => Ok(dt),
            other => Err(DomainError::conversion(
                "DateTime<Utc>",
                get_variant_name(&other),
            )),
        }
    }
}

// --- Date ---
impl TryFrom<SqlValue> for NaiveDate {
    type Error = DomainError;

    fn try_from(val: SqlValue) -> Result<Self, Self::Error> {
        match val {
            SqlValue::Date(d) => Ok(d),
            other => Err(DomainError::conversion(
                "NaiveDate",
                get_variant_name(&other),
            )),
        }
    }
}

// --- Time ---
impl TryFrom<SqlValue> for NaiveTime {
    type Error = DomainError;

    fn try_from(val: SqlValue) -> Result<Self, Self::Error> {
        match val {
            SqlValue::Time(t) => Ok(t),
            other => Err(DomainError::conversion(
                "NaiveTime",
                get_variant_name(&other),
            )),
        }
    }
}

// --- Bytes ---
impl TryFrom<SqlValue> for Vec<u8> {
    type Error = DomainError;

    fn try_from(val: SqlValue) -> Result<Self, Self::Error> {
        match val {
            SqlValue::Bytes(b) => Ok(b),
            other => Err(DomainError::conversion("Vec<u8>", get_variant_name(&other))),
        }
    }
}

// --- Support Option<T> untuk kolom yang BISA NULL (Nullable Column) ---
impl<T> TryFrom<SqlValue> for Option<T>
where
    T: TryFrom<SqlValue, Error = DomainError>,
{
    type Error = DomainError;

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
        SqlValue::Null => "Null",
        SqlValue::Int(_) => "Int",
        SqlValue::Float(_) => "Float",
        SqlValue::Text(_) => "Text",
        SqlValue::Bool(_) => "Bool",
        SqlValue::Timestamp(_) => "Timestamp",
        SqlValue::Date(_) => "Date",
        SqlValue::Time(_) => "Time",
        SqlValue::Bytes(_) => "Bytes",
        SqlValue::Enum {
            type_name: _,
            value: _,
        } => "Enum",
        SqlValue::Custom {
            type_name: _,
            value: _,
        } => "Custom",
    }
}

// =============================================================================
// 5. IMPLEMENTASI EKSTRAKSI UTC UNTUK CLIENT
// =============================================================================

/// memecah UTC sebagai kumpulan data spesifik
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

    /// String lengkap untuk tanggal, waktu, dan zona.
    pub fn to_receipt_string(&self) -> String {
        format!(
            "{} {} {}",
            self.formatted_date(),
            self.formatted_time(),
            self.zona
        )
    }
}

impl SqlValue {
    /// Mencoba mengonversi (cast) nilai SqlValue ke target SqlType
    pub fn try_cast_to(&self, target_type: &SqlType) -> Result<SqlValue, DomainError> {
        if self.is_null() {
            return Ok(SqlValue::Null);
        }

        match (self, target_type) {
            // 1. Same type / No-op
            (SqlValue::Int(v), SqlType::Int) => Ok(SqlValue::Int(*v)),
            (SqlValue::Float(v), SqlType::Float) => Ok(SqlValue::Float(*v)),
            (SqlValue::Text(v), SqlType::Text) => Ok(SqlValue::Text(v.clone())),
            (SqlValue::Bool(v), SqlType::Bool) => Ok(SqlValue::Bool(*v)),

            // 2. Int <-> Float (Menggunakan OrderedFloat)
            (SqlValue::Int(v), SqlType::Float) => Ok(SqlValue::Float(OrderedFloat(*v as f64))),
            (SqlValue::Float(v), SqlType::Int) => Ok(SqlValue::Int(v.into_inner() as i64)),

            // 3. Int/Float/Bool -> Text
            (SqlValue::Int(v), SqlType::Text) => Ok(SqlValue::Text(v.to_string())),
            (SqlValue::Float(v), SqlType::Text) => Ok(SqlValue::Text(v.to_string())),
            (SqlValue::Bool(v), SqlType::Text) => Ok(SqlValue::Text(v.to_string())),

            // 4. Text -> Int/Float/Bool (Parsing)
            (SqlValue::Text(s), SqlType::Int) => {
                s.trim().parse::<i64>().map(SqlValue::Int).map_err(|_| {
                    DomainError::EvaluationError(format!("Gagal mengonversi teks '{s}' ke Int"))
                })
            }

            (SqlValue::Text(s), SqlType::Float) => s
                .trim()
                .parse::<f64>()
                .map(|f| SqlValue::Float(OrderedFloat(f)))
                .map_err(|_| {
                    DomainError::EvaluationError(format!("Gagal mengonversi teks '{s}' ke Float"))
                }),

            (SqlValue::Text(s), SqlType::Bool) => match s.trim().to_lowercase().as_str() {
                "true" | "1" | "t" => Ok(SqlValue::Bool(true)),
                "false" | "0" | "f" => Ok(SqlValue::Bool(false)),
                _ => Err(DomainError::EvaluationError(format!(
                    "Gagal mengonversi teks '{s}' ke Bool"
                ))),
            },

            // Text -> Timestamp/Date/Time
            (SqlValue::Text(s), SqlType::Date) => Self::parse_date(s),
            (SqlValue::Text(s), SqlType::Time) => Self::parse_time(s),

            // Enum Validation Cast
            // Text -> Enum Cast:
            (SqlValue::Text(s), SqlType::Enum { name, variants }) => {
                if variants.contains(s) {
                    Ok(SqlValue::Enum {
                        type_name: name.clone(),
                        value: s.clone(),
                    })
                } else {
                    Err(DomainError::EvaluationError(format!(
                        "Nilai '{s}' tidak valid untuk varian Enum '{name}'"
                    )))
                }
            }

            // Text -> Custom Cast:
            (SqlValue::Text(s), SqlType::Custom(type_name)) => Ok(SqlValue::Custom {
                type_name: type_name.clone(),
                value: s.clone(),
            }),

            _ => Err(DomainError::EvaluationError(format!(
                "Konversi tipe data dari '{:?}' ke '{:?}' tidak didukung",
                self, target_type
            ))),
        }
    }
}
