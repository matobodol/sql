use chrono::{DateTime, Local, NaiveDate, NaiveTime, Utc};
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{DomainError, SqlBool, SqlType};

/// Representasi Nilai Data SQL di Runtime dengan Zero-Copy Cheap Clone (O(1)).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SqlValue {
    /// Representasi tidak ada nilai
    Null,
    /// Bilangan bulat i64
    Int(i64),
    /// Bilangan pecahan f64
    Float(OrderedFloat<f64>),
    /// Sql String dengan O(1) Arc clone
    Text(Arc<str>),
    /// Sql boolean
    Bool(bool),

    /// Tipe file atau media dengan O(1) Arc clone
    Bytes(Arc<[u8]>),

    /// Timestamp, selalu disimpan dalam UTC (Single Source of Truth)
    Timestamp(DateTime<Utc>),
    Date(NaiveDate),
    Time(NaiveTime),

    // UDT User-Defined Tipe atau Domain Type.
    /// Representasi runtime nilai Enum: (Nama Type Enum, Value Variant)
    Enum {
        type_name: Arc<str>,
        value: Arc<str>,
    },

    /// Representasi runtime tipe kustom/domain khusus: (Nama Type Custom, Value Raw)
    Custom {
        type_name: Arc<str>,
        value: Arc<str>,
    },
}

impl SqlValue {
    // --- HELPER CONSTRUCTOR ZERO-COPY ---

    /// Helper instan untuk membuat SqlValue::Text dari &str tanpa boilerplate alokasi berlebih
    pub fn text(s: &str) -> Self {
        SqlValue::Text(Arc::from(s))
    }

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
                DomainError::invalid_expr(format!(
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
                DomainError::invalid_expr(format!(
                    "Format waktu '{input}' salah (Gunakan HH:MM:SS atau HH:MM): {e}"
                ))
            })
    }

    // --- VALIDASI TIPE ---
    /// Validasi tipe antara value dan schema mengembalikan boolean.
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
                type_name.as_ref() == name && variants.contains(&value.to_string())
            }

            // Validasi Custom: type_name harus cocok
            (SqlValue::Custom { type_name, .. }, SqlType::Custom(expected_type)) => {
                type_name.as_ref() == expected_type
            }

            _ => false,
        }
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
            (SqlValue::Text(v), SqlType::Text) => Ok(SqlValue::Text(Arc::clone(v))),
            (SqlValue::Bool(v), SqlType::Bool) => Ok(SqlValue::Bool(*v)),

            // 2. Int <-> Float
            (SqlValue::Int(v), SqlType::Float) => Ok(SqlValue::Float(OrderedFloat(*v as f64))),
            (SqlValue::Float(v), SqlType::Int) => Ok(SqlValue::Int(v.into_inner() as i64)),

            // 3. Int/Float/Bool -> Text (Zero-Copy Arc)
            (SqlValue::Int(v), SqlType::Text) => Ok(SqlValue::Text(Arc::from(v.to_string()))),
            (SqlValue::Float(v), SqlType::Text) => Ok(SqlValue::Text(Arc::from(v.to_string()))),
            (SqlValue::Bool(v), SqlType::Text) => Ok(SqlValue::Text(Arc::from(v.to_string()))),

            // 4. Text -> Int/Float/Bool (Parsing)
            (SqlValue::Text(s), SqlType::Int) => {
                s.trim().parse::<i64>().map(SqlValue::Int).map_err(|_| {
                    DomainError::eval_error(format!("Gagal mengonversi teks '{s}' ke Int"))
                })
            }

            (SqlValue::Text(s), SqlType::Float) => s
                .trim()
                .parse::<f64>()
                .map(|f| SqlValue::Float(OrderedFloat(f)))
                .map_err(|_| {
                    DomainError::eval_error(format!("Gagal mengonversi teks '{s}' ke Float"))
                }),

            (SqlValue::Text(s), SqlType::Bool) => match s.trim().to_lowercase().as_str() {
                "true" | "1" | "t" => Ok(SqlValue::Bool(true)),
                "false" | "0" | "f" => Ok(SqlValue::Bool(false)),
                _ => Err(DomainError::eval_error(format!(
                    "Gagal mengonversi teks '{s}' ke Bool"
                ))),
            },

            // Text -> Timestamp/Date/Time
            (SqlValue::Text(s), SqlType::Date) => Self::parse_date(s),
            (SqlValue::Text(s), SqlType::Time) => Self::parse_time(s),

            // Enum Validation Cast
            (SqlValue::Text(s), SqlType::Enum { name, variants }) => {
                if variants.contains(&s.to_string()) {
                    Ok(SqlValue::Enum {
                        type_name: Arc::from(name.as_str()),
                        value: Arc::clone(s),
                    })
                } else {
                    Err(DomainError::eval_error(format!(
                        "Nilai '{s}' tidak valid untuk varian Enum '{name}'"
                    )))
                }
            }

            // Text -> Custom Cast
            (SqlValue::Text(s), SqlType::Custom(type_name)) => Ok(SqlValue::Custom {
                type_name: Arc::from(type_name.as_str()),
                value: Arc::clone(s),
            }),

            _ => Err(DomainError::eval_error(format!(
                "Konversi tipe data dari '{:?}' ke '{:?}' tidak didukung",
                self, target_type
            ))),
        }
    }
}

use std::cmp::Ordering;
use std::ops::Not;

impl SqlValue {
    /// Helper untuk memeriksa apakah nilai bernilai NULL
    #[inline]
    pub fn is_null(&self) -> bool {
        matches!(self, SqlValue::Null)
    }

    /// EQUAL (=)
    pub fn eq(&self, other: &SqlValue) -> SqlBool {
        if self.is_null() || other.is_null() {
            SqlBool::Unknown
        } else {
            SqlBool::from(self.cmp(other) == Ordering::Equal)
        }
    }

    /// NOT EQUAL (!= atau <>)
    pub fn noteq(&self, other: &Self) -> SqlBool {
        self.eq(other).not()
    }

    /// GREATER THAN (>)
    pub fn gt(&self, other: &Self) -> SqlBool {
        if self.is_null() || other.is_null() {
            SqlBool::Unknown
        } else {
            SqlBool::from(self.cmp(other) == Ordering::Greater)
        }
    }

    /// LESS THAN (<)
    pub fn lt(&self, other: &Self) -> SqlBool {
        if self.is_null() || other.is_null() {
            SqlBool::Unknown
        } else {
            SqlBool::from(self.cmp(other) == Ordering::Less)
        }
    }

    /// GREATER THAN OR EQUAL (>=)
    pub fn gteq(&self, other: &Self) -> SqlBool {
        if self.is_null() || other.is_null() {
            SqlBool::Unknown
        } else {
            let cmp = matches!(self.cmp(other), Ordering::Greater | Ordering::Equal);
            SqlBool::from(cmp)
        }
    }

    /// LESS THAN OR EQUAL (<=)
    pub fn lteq(&self, other: &Self) -> SqlBool {
        if self.is_null() || other.is_null() {
            SqlBool::Unknown
        } else {
            let cmp = matches!(self.cmp(other), Ordering::Less | Ordering::Equal);
            SqlBool::from(cmp)
        }
    }

    /// OPERATOR LOGIKA AND
    pub fn and(&self, other: &Self) -> Result<SqlBool, DomainError> {
        let r = SqlBool::try_from(self)?;
        let l = SqlBool::try_from(other)?;
        Ok(l.and(r))
    }

    /// OPERATOR LOGIKA OR
    pub fn or(&self, other: &Self) -> Result<SqlBool, DomainError> {
        let l = SqlBool::try_from(self)?;
        let r = SqlBool::try_from(other)?;
        Ok(l.or(r))
    }
}

// --- IMPLEMENTASI ORD MANUAL UNTUK BTREE & COMPARISON ---

fn variant_index(val: &SqlValue) -> usize {
    match val {
        SqlValue::Null => 0,
        SqlValue::Int(_) => 1,
        SqlValue::Float(_) => 2,
        SqlValue::Text(_) => 3,
        SqlValue::Bool(_) => 4,
        SqlValue::Bytes(_) => 5,
        SqlValue::Timestamp(_) => 6,
        SqlValue::Date(_) => 7,
        SqlValue::Time(_) => 8,
        SqlValue::Enum { .. } => 9,
        SqlValue::Custom { .. } => 10,
    }
}

impl Ord for SqlValue {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (SqlValue::Null, SqlValue::Null) => Ordering::Equal,
            (SqlValue::Null, _) => Ordering::Less,
            (_, SqlValue::Null) => Ordering::Greater,

            (SqlValue::Int(x), SqlValue::Int(y)) => x.cmp(y),
            (SqlValue::Float(x), SqlValue::Float(y)) => x.cmp(y),
            (SqlValue::Int(x), SqlValue::Float(y)) => ordered_float::OrderedFloat(*x as f64).cmp(y),
            (SqlValue::Float(x), SqlValue::Int(y)) => {
                x.cmp(&ordered_float::OrderedFloat(*y as f64))
            }

            (SqlValue::Text(x), SqlValue::Text(y)) => x.cmp(y),
            (SqlValue::Bool(x), SqlValue::Bool(y)) => x.cmp(y),
            (SqlValue::Timestamp(x), SqlValue::Timestamp(y)) => x.cmp(y),
            (SqlValue::Date(x), SqlValue::Date(y)) => x.cmp(y),
            (SqlValue::Time(x), SqlValue::Time(y)) => x.cmp(y),
            (SqlValue::Bytes(x), SqlValue::Bytes(y)) => x.cmp(y),

            (
                SqlValue::Enum {
                    type_name: t1,
                    value: v1,
                },
                SqlValue::Enum {
                    type_name: t2,
                    value: v2,
                },
            ) => t1.cmp(t2).then_with(|| v1.cmp(v2)),
            (
                SqlValue::Custom {
                    type_name: t1,
                    value: v1,
                },
                SqlValue::Custom {
                    type_name: t2,
                    value: v2,
                },
            ) => t1.cmp(t2).then_with(|| v1.cmp(v2)),

            (a, b) => variant_index(a).cmp(&variant_index(b)),
        }
    }
}

impl PartialOrd for SqlValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// =============================================================================
// IMPLEMENTASI `From` (Mengubah Tipe Rust -> SqlValue)
// =============================================================================

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
        SqlValue::Text(Arc::from(v))
    }
}
impl From<&str> for SqlValue {
    fn from(v: &str) -> Self {
        SqlValue::Text(Arc::from(v))
    }
}
impl From<Arc<str>> for SqlValue {
    fn from(v: Arc<str>) -> Self {
        SqlValue::Text(v)
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
        SqlValue::Bytes(Arc::from(v))
    }
}
impl From<&[u8]> for SqlValue {
    fn from(v: &[u8]) -> Self {
        SqlValue::Bytes(Arc::from(v))
    }
}

// Support otomatis untuk Option<T>
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
// IMPLEMENTASI `TryFrom` (Mengekstrak SqlValue -> Tipe Rust Asli)
// =============================================================================

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

impl TryFrom<SqlValue> for f64 {
    type Error = DomainError;

    fn try_from(val: SqlValue) -> Result<Self, Self::Error> {
        match val {
            SqlValue::Float(f) => Ok(f.into_inner()),
            other => Err(DomainError::conversion("f64", get_variant_name(&other))),
        }
    }
}

impl TryFrom<SqlValue> for String {
    type Error = DomainError;

    fn try_from(val: SqlValue) -> Result<Self, Self::Error> {
        match val {
            SqlValue::Text(s) => Ok(s.to_string()),
            other => Err(DomainError::conversion("String", get_variant_name(&other))),
        }
    }
}

impl TryFrom<SqlValue> for Arc<str> {
    type Error = DomainError;

    fn try_from(val: SqlValue) -> Result<Self, Self::Error> {
        match val {
            SqlValue::Text(s) => Ok(s),
            other => Err(DomainError::conversion(
                "Arc<str>",
                get_variant_name(&other),
            )),
        }
    }
}

impl TryFrom<SqlValue> for bool {
    type Error = DomainError;

    fn try_from(val: SqlValue) -> Result<Self, Self::Error> {
        match val {
            SqlValue::Bool(b) => Ok(b),
            other => Err(DomainError::conversion("bool", get_variant_name(&other))),
        }
    }
}

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

impl TryFrom<SqlValue> for Vec<u8> {
    type Error = DomainError;

    fn try_from(val: SqlValue) -> Result<Self, Self::Error> {
        match val {
            SqlValue::Bytes(b) => Ok(b.to_vec()),
            other => Err(DomainError::conversion("Vec<u8>", get_variant_name(&other))),
        }
    }
}

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
        SqlValue::Enum { .. } => "Enum",
        SqlValue::Custom { .. } => "Custom",
    }
}

// =============================================================================
// EKSTRAKSI UTC UNTUK CLIENT
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EkstrakTimeStamp {
    pub utc: DateTime<Utc>,
    pub zona: String,
    pub date: NaiveDate,
    pub time: NaiveTime,
}

impl EkstrakTimeStamp {
    pub fn from_utc_local(utc_dt: DateTime<Utc>) -> Self {
        let local_dt: DateTime<Local> = DateTime::from(utc_dt);

        Self {
            utc: utc_dt,
            zona: local_dt.format("%Z").to_string(),
            date: local_dt.date_naive(),
            time: local_dt.time(),
        }
    }

    pub fn formatted_date(&self) -> String {
        self.date.format("%Y-%m-%d").to_string()
    }

    pub fn formatted_time(&self) -> String {
        self.time.format("%H:%M:%S").to_string()
    }

    pub fn to_receipt_string(&self) -> String {
        format!(
            "{} {} {}",
            self.formatted_date(),
            self.formatted_time(),
            self.zona
        )
    }
}
