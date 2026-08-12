use chrono::{DateTime, Local, NaiveDate, NaiveTime, Utc};
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{Bool3VL, DataType, DomainError};

/// Representasi Nilai Data SQL di Runtime dengan Zero-Copy Cheap Clone (O(1)).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ValueType {
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

impl ValueType {
    // --- HELPER CONSTRUCTOR ZERO-COPY ---

    /// Helper instan untuk membuat SqlValue::Text dari &str tanpa boilerplate alokasi berlebih
    pub fn text(s: &str) -> Self {
        ValueType::Text(Arc::from(s))
    }

    // --- CONSTRUCTOR HELPER (AUTO DATE/TIME DARI CHRONO UTCTIMESTAMP) ---

    /// Mengambil Timestamp saat ini dalam UTC
    pub fn now() -> Self {
        ValueType::Timestamp(Utc::now())
    }

    /// Auto-extract komponen DATE (Lokal) langsung dari DateTime<Utc>
    pub fn date_from_datetime(dt: DateTime<Utc>) -> Self {
        let local_dt: DateTime<Local> = DateTime::from(dt);
        ValueType::Date(local_dt.date_naive())
    }

    /// Auto-extract komponen TIME (Lokal) langsung dari DateTime<Utc>
    pub fn time_from_datetime(dt: DateTime<Utc>) -> Self {
        let local_dt: DateTime<Local> = DateTime::from(dt);
        ValueType::Time(local_dt.time())
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
            .map(ValueType::Date)
            .map_err(|e| {
                DomainError::invalid_expr(format!(
                    "Format tanggal '{input}' salah (Gunakan YYYY-MM-DD): {e}"
                ))
            })
    }

    /// Parse manual dari String ke Time ("HH:MM:SS" atau "HH:MM")
    pub fn parse_time(input: &str) -> Result<Self, DomainError> {
        if let Ok(t) = NaiveTime::parse_from_str(input, "%H:%M:%S") {
            return Ok(ValueType::Time(t));
        }

        NaiveTime::parse_from_str(input, "%H:%M")
            .map(ValueType::Time)
            .map_err(|e| {
                DomainError::invalid_expr(format!(
                    "Format waktu '{input}' salah (Gunakan HH:MM:SS atau HH:MM): {e}"
                ))
            })
    }

    // --- VALIDASI TIPE ---
    /// Validasi tipe antara value dan schema mengembalikan boolean.
    pub fn matches_type(&self, sql_type: &DataType) -> bool {
        match (self, sql_type) {
            (ValueType::Null, _) => true,
            (ValueType::Int(_), DataType::Int) => true,
            (ValueType::Float(_), DataType::Float) => true,
            (ValueType::Text(_), DataType::Text) => true,
            (ValueType::Bool(_), DataType::Bool) => true,
            (ValueType::Timestamp(_), DataType::Timestamp) => true,
            (ValueType::Date(_), DataType::Date) => true,
            (ValueType::Time(_), DataType::Time) => true,
            (ValueType::Bytes(_), DataType::Bytes) => true,

            // Validasi Enum: defined/name cocok DAN value ada di daftar variants
            (ValueType::Enum { type_name, value }, DataType::Enum { name, variants }) => {
                type_name.as_ref() == name && variants.contains(&value.to_string())
            }

            // Validasi Custom: type_name harus cocok
            (ValueType::Custom { type_name, .. }, DataType::Custom(expected_type)) => {
                type_name.as_ref() == expected_type
            }

            _ => false,
        }
    }
}

impl ValueType {
    /// Mencoba mengonversi (cast) nilai SqlValue ke target SqlType
    pub fn try_cast_to(&self, target_type: &DataType) -> Result<ValueType, DomainError> {
        if self.is_null() {
            return Ok(ValueType::Null);
        }

        match (self, target_type) {
            // 1. Same type / No-op
            (ValueType::Int(v), DataType::Int) => Ok(ValueType::Int(*v)),
            (ValueType::Float(v), DataType::Float) => Ok(ValueType::Float(*v)),
            (ValueType::Text(v), DataType::Text) => Ok(ValueType::Text(Arc::clone(v))),
            (ValueType::Bool(v), DataType::Bool) => Ok(ValueType::Bool(*v)),

            // 2. Int <-> Float
            (ValueType::Int(v), DataType::Float) => Ok(ValueType::Float(OrderedFloat(*v as f64))),
            (ValueType::Float(v), DataType::Int) => Ok(ValueType::Int(v.into_inner() as i64)),

            // 3. Int/Float/Bool -> Text (Zero-Copy Arc)
            (ValueType::Int(v), DataType::Text) => Ok(ValueType::Text(Arc::from(v.to_string()))),
            (ValueType::Float(v), DataType::Text) => Ok(ValueType::Text(Arc::from(v.to_string()))),
            (ValueType::Bool(v), DataType::Text) => Ok(ValueType::Text(Arc::from(v.to_string()))),

            // 4. Text -> Int/Float/Bool (Parsing)
            (ValueType::Text(s), DataType::Int) => {
                s.trim().parse::<i64>().map(ValueType::Int).map_err(|_| {
                    DomainError::eval_error(format!("Gagal mengonversi teks '{s}' ke Int"))
                })
            }

            (ValueType::Text(s), DataType::Float) => s
                .trim()
                .parse::<f64>()
                .map(|f| ValueType::Float(OrderedFloat(f)))
                .map_err(|_| {
                    DomainError::eval_error(format!("Gagal mengonversi teks '{s}' ke Float"))
                }),

            (ValueType::Text(s), DataType::Bool) => match s.trim() {
                "true" | "1" | "t" => Ok(ValueType::Bool(true)),
                "false" | "0" | "f" => Ok(ValueType::Bool(false)),
                _ => Err(DomainError::eval_error(format!(
                    "Gagal mengonversi teks '{s}' ke Bool"
                ))),
            },

            // Text -> Timestamp/Date/Time
            (ValueType::Text(s), DataType::Date) => Self::parse_date(s),
            (ValueType::Text(s), DataType::Time) => Self::parse_time(s),

            // Enum Validation Cast
            (ValueType::Text(s), DataType::Enum { name, variants }) => {
                if variants.contains(&s.to_string()) {
                    Ok(ValueType::Enum {
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
            (ValueType::Text(s), DataType::Custom(type_name)) => Ok(ValueType::Custom {
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

impl ValueType {
    /// Helper untuk memeriksa apakah nilai bernilai NULL
    #[inline]
    pub fn is_null(&self) -> bool {
        matches!(self, ValueType::Null)
    }

    /// EQUAL (=)
    pub fn eq(&self, other: &ValueType) -> Bool3VL {
        if self.is_null() || other.is_null() {
            Bool3VL::Unknown
        } else {
            Bool3VL::from(self.cmp(other) == Ordering::Equal)
        }
    }

    /// NOT EQUAL (!= atau <>)
    pub fn noteq(&self, other: &Self) -> Bool3VL {
        self.eq(other).not()
    }

    /// GREATER THAN (>)
    pub fn gt(&self, other: &Self) -> Bool3VL {
        if self.is_null() || other.is_null() {
            Bool3VL::Unknown
        } else {
            Bool3VL::from(self.cmp(other) == Ordering::Greater)
        }
    }

    /// LESS THAN (<)
    pub fn lt(&self, other: &Self) -> Bool3VL {
        if self.is_null() || other.is_null() {
            Bool3VL::Unknown
        } else {
            Bool3VL::from(self.cmp(other) == Ordering::Less)
        }
    }

    /// GREATER THAN OR EQUAL (>=)
    pub fn gteq(&self, other: &Self) -> Bool3VL {
        if self.is_null() || other.is_null() {
            Bool3VL::Unknown
        } else {
            let cmp = matches!(self.cmp(other), Ordering::Greater | Ordering::Equal);
            Bool3VL::from(cmp)
        }
    }

    /// LESS THAN OR EQUAL (<=)
    pub fn lteq(&self, other: &Self) -> Bool3VL {
        if self.is_null() || other.is_null() {
            Bool3VL::Unknown
        } else {
            let cmp = matches!(self.cmp(other), Ordering::Less | Ordering::Equal);
            Bool3VL::from(cmp)
        }
    }

    /// OPERATOR LOGIKA AND
    pub fn and(&self, other: &Self) -> Result<Bool3VL, DomainError> {
        let r = Bool3VL::try_from(self)?;
        let l = Bool3VL::try_from(other)?;
        Ok(l.and(r))
    }

    /// OPERATOR LOGIKA OR
    pub fn or(&self, other: &Self) -> Result<Bool3VL, DomainError> {
        let l = Bool3VL::try_from(self)?;
        let r = Bool3VL::try_from(other)?;
        Ok(l.or(r))
    }
}

// --- IMPLEMENTASI ORD MANUAL UNTUK BTREE & COMPARISON ---

fn variant_index(val: &ValueType) -> usize {
    match val {
        ValueType::Null => 0,
        ValueType::Int(_) => 1,
        ValueType::Float(_) => 2,
        ValueType::Text(_) => 3,
        ValueType::Bool(_) => 4,
        ValueType::Bytes(_) => 5,
        ValueType::Timestamp(_) => 6,
        ValueType::Date(_) => 7,
        ValueType::Time(_) => 8,
        ValueType::Enum { .. } => 9,
        ValueType::Custom { .. } => 10,
    }
}

impl Ord for ValueType {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (ValueType::Null, ValueType::Null) => Ordering::Equal,
            (ValueType::Null, _) => Ordering::Less,
            (_, ValueType::Null) => Ordering::Greater,

            (ValueType::Int(x), ValueType::Int(y)) => x.cmp(y),
            (ValueType::Float(x), ValueType::Float(y)) => x.cmp(y),
            (ValueType::Int(x), ValueType::Float(y)) => {
                ordered_float::OrderedFloat(*x as f64).cmp(y)
            }
            (ValueType::Float(x), ValueType::Int(y)) => {
                x.cmp(&ordered_float::OrderedFloat(*y as f64))
            }

            (ValueType::Text(x), ValueType::Text(y)) => x.cmp(y),
            (ValueType::Bool(x), ValueType::Bool(y)) => x.cmp(y),
            (ValueType::Timestamp(x), ValueType::Timestamp(y)) => x.cmp(y),
            (ValueType::Date(x), ValueType::Date(y)) => x.cmp(y),
            (ValueType::Time(x), ValueType::Time(y)) => x.cmp(y),
            (ValueType::Bytes(x), ValueType::Bytes(y)) => x.cmp(y),

            (
                ValueType::Enum {
                    type_name: t1,
                    value: v1,
                },
                ValueType::Enum {
                    type_name: t2,
                    value: v2,
                },
            ) => t1.cmp(t2).then_with(|| v1.cmp(v2)),
            (
                ValueType::Custom {
                    type_name: t1,
                    value: v1,
                },
                ValueType::Custom {
                    type_name: t2,
                    value: v2,
                },
            ) => t1.cmp(t2).then_with(|| v1.cmp(v2)),

            (a, b) => variant_index(a).cmp(&variant_index(b)),
        }
    }
}

impl PartialOrd for ValueType {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// =============================================================================
// IMPLEMENTASI `From` (Mengubah Tipe Rust -> SqlValue)
// =============================================================================

impl From<&Bool3VL> for ValueType {
    fn from(sb: &Bool3VL) -> Self {
        match sb {
            Bool3VL::True => ValueType::Bool(true),
            Bool3VL::False => ValueType::Bool(false),
            Bool3VL::Unknown => ValueType::Null,
        }
    }
}

impl From<Bool3VL> for ValueType {
    fn from(sb: Bool3VL) -> Self {
        ValueType::from(&sb)
    }
}

impl From<i64> for ValueType {
    fn from(v: i64) -> Self {
        ValueType::Int(v)
    }
}
impl From<i32> for ValueType {
    fn from(v: i32) -> Self {
        ValueType::Int(v as i64)
    }
}
impl From<usize> for ValueType {
    fn from(v: usize) -> Self {
        ValueType::Int(v as i64)
    }
}
impl From<f64> for ValueType {
    fn from(v: f64) -> Self {
        ValueType::Float(OrderedFloat(v))
    }
}
impl From<f32> for ValueType {
    fn from(v: f32) -> Self {
        ValueType::Float(OrderedFloat(v as f64))
    }
}
impl From<String> for ValueType {
    fn from(v: String) -> Self {
        ValueType::Text(Arc::from(v))
    }
}
impl From<&str> for ValueType {
    fn from(v: &str) -> Self {
        ValueType::Text(Arc::from(v))
    }
}
impl From<Arc<str>> for ValueType {
    fn from(v: Arc<str>) -> Self {
        ValueType::Text(v)
    }
}
impl From<bool> for ValueType {
    fn from(v: bool) -> Self {
        ValueType::Bool(v)
    }
}
impl From<DateTime<Utc>> for ValueType {
    fn from(v: DateTime<Utc>) -> Self {
        ValueType::Timestamp(v)
    }
}
impl From<NaiveDate> for ValueType {
    fn from(v: NaiveDate) -> Self {
        ValueType::Date(v)
    }
}
impl From<NaiveTime> for ValueType {
    fn from(v: NaiveTime) -> Self {
        ValueType::Time(v)
    }
}
impl From<Vec<u8>> for ValueType {
    fn from(v: Vec<u8>) -> Self {
        ValueType::Bytes(Arc::from(v))
    }
}
impl From<&[u8]> for ValueType {
    fn from(v: &[u8]) -> Self {
        ValueType::Bytes(Arc::from(v))
    }
}

// Support otomatis untuk Option<T>
impl<T> From<Option<T>> for ValueType
where
    ValueType: From<T>,
{
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(v) => ValueType::from(v),
            None => ValueType::Null,
        }
    }
}

// =============================================================================
// IMPLEMENTASI `TryFrom` (Mengekstrak SqlValue -> Tipe Rust Asli)
// =============================================================================

impl TryFrom<ValueType> for i64 {
    type Error = DomainError;

    fn try_from(val: ValueType) -> Result<Self, Self::Error> {
        match val {
            ValueType::Int(n) => Ok(n),
            other => Err(DomainError::conversion("i64", get_variant_name(&other))),
        }
    }
}

impl TryFrom<ValueType> for i32 {
    type Error = DomainError;

    fn try_from(val: ValueType) -> Result<Self, Self::Error> {
        match val {
            ValueType::Int(n) => n
                .try_into()
                .map_err(|_| DomainError::conversion("i32 (out of bounds)", "i64")),
            other => Err(DomainError::conversion("i32", get_variant_name(&other))),
        }
    }
}

impl TryFrom<ValueType> for f64 {
    type Error = DomainError;

    fn try_from(val: ValueType) -> Result<Self, Self::Error> {
        match val {
            ValueType::Float(f) => Ok(f.into_inner()),
            other => Err(DomainError::conversion("f64", get_variant_name(&other))),
        }
    }
}

impl TryFrom<ValueType> for String {
    type Error = DomainError;

    fn try_from(val: ValueType) -> Result<Self, Self::Error> {
        match val {
            ValueType::Text(s) => Ok(s.to_string()),
            other => Err(DomainError::conversion("String", get_variant_name(&other))),
        }
    }
}

impl TryFrom<ValueType> for Arc<str> {
    type Error = DomainError;

    fn try_from(val: ValueType) -> Result<Self, Self::Error> {
        match val {
            ValueType::Text(s) => Ok(s),
            other => Err(DomainError::conversion(
                "Arc<str>",
                get_variant_name(&other),
            )),
        }
    }
}

impl TryFrom<ValueType> for bool {
    type Error = DomainError;

    fn try_from(val: ValueType) -> Result<Self, Self::Error> {
        match val {
            ValueType::Bool(b) => Ok(b),
            other => Err(DomainError::conversion("bool", get_variant_name(&other))),
        }
    }
}

impl TryFrom<ValueType> for DateTime<Utc> {
    type Error = DomainError;

    fn try_from(val: ValueType) -> Result<Self, Self::Error> {
        match val {
            ValueType::Timestamp(dt) => Ok(dt),
            other => Err(DomainError::conversion(
                "DateTime<Utc>",
                get_variant_name(&other),
            )),
        }
    }
}

impl TryFrom<ValueType> for NaiveDate {
    type Error = DomainError;

    fn try_from(val: ValueType) -> Result<Self, Self::Error> {
        match val {
            ValueType::Date(d) => Ok(d),
            other => Err(DomainError::conversion(
                "NaiveDate",
                get_variant_name(&other),
            )),
        }
    }
}

impl TryFrom<ValueType> for NaiveTime {
    type Error = DomainError;

    fn try_from(val: ValueType) -> Result<Self, Self::Error> {
        match val {
            ValueType::Time(t) => Ok(t),
            other => Err(DomainError::conversion(
                "NaiveTime",
                get_variant_name(&other),
            )),
        }
    }
}

impl TryFrom<ValueType> for Vec<u8> {
    type Error = DomainError;

    fn try_from(val: ValueType) -> Result<Self, Self::Error> {
        match val {
            ValueType::Bytes(b) => Ok(b.to_vec()),
            other => Err(DomainError::conversion("Vec<u8>", get_variant_name(&other))),
        }
    }
}

impl<T> TryFrom<ValueType> for Option<T>
where
    T: TryFrom<ValueType, Error = DomainError>,
{
    type Error = DomainError;

    fn try_from(val: ValueType) -> Result<Self, Self::Error> {
        match val {
            ValueType::Null => Ok(None),
            other => T::try_from(other).map(Some),
        }
    }
}

fn get_variant_name(val: &ValueType) -> &'static str {
    match val {
        ValueType::Null => "Null",
        ValueType::Int(_) => "Int",
        ValueType::Float(_) => "Float",
        ValueType::Text(_) => "Text",
        ValueType::Bool(_) => "Bool",
        ValueType::Timestamp(_) => "Timestamp",
        ValueType::Date(_) => "Date",
        ValueType::Time(_) => "Time",
        ValueType::Bytes(_) => "Bytes",
        ValueType::Enum { .. } => "Enum",
        ValueType::Custom { .. } => "Custom",
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
