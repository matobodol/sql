use ordered_float::OrderedFloat;
use std::cmp::Ordering;
use std::ops::Not;

use crate::{DomainError, SqlBool, SqlValue};

/// Helper internal untuk menentukan urutan varian enum saat perbandingan beda tipe
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
        SqlValue::Enum { type_name: _, .. } => 9,
        SqlValue::Custom { type_name: _, .. } => 10,
    }
}

impl SqlValue {
    /// Apakah Value adalah null
    pub fn is_null(&self) -> bool {
        matches!(self, SqlValue::Null)
    }

    /// EQUAL (=)
    pub fn eq(&self, other: &Self) -> SqlBool {
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

//
// -- IMPLEMENTASI ORD MANUAL --
//
impl Ord for SqlValue {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            // Aturan SQL NULL: NULL dianggap paling kecil (NULLS FIRST)
            (SqlValue::Null, SqlValue::Null) => Ordering::Equal,
            (SqlValue::Null, _) => Ordering::Less,
            (_, SqlValue::Null) => Ordering::Greater,

            // Perbandingan Tipe Sama Primitif
            (SqlValue::Int(x), SqlValue::Int(y)) => x.cmp(y),
            (SqlValue::Float(x), SqlValue::Float(y)) => x.cmp(y),

            // Perbandingan Silang Tipe Angka (Int <-> Float)
            (SqlValue::Int(x), SqlValue::Float(y)) => OrderedFloat(*x as f64).cmp(y),
            (SqlValue::Float(x), SqlValue::Int(y)) => x.cmp(&OrderedFloat(*y as f64)),

            // Tipe Sama Lainnya
            (SqlValue::Text(x), SqlValue::Text(y)) => x.cmp(y),
            (SqlValue::Bool(x), SqlValue::Bool(y)) => x.cmp(y),
            (SqlValue::Timestamp(x), SqlValue::Timestamp(y)) => x.cmp(y),
            (SqlValue::Date(x), SqlValue::Date(y)) => x.cmp(y),
            (SqlValue::Time(x), SqlValue::Time(y)) => x.cmp(y),
            (SqlValue::Bytes(x), SqlValue::Bytes(y)) => x.cmp(y),

            // Perbandingan Enum Type Sama (Cek 'defined' dulu, baru 'value')
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

            // Perbandingan Custom Type Sama (Cek 'type_name' dulu, baru 'value')
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

            // Fallback ANSI SQL: Tipe beda total diurutkan berdasarkan indeks varian
            (a, b) => variant_index(a).cmp(&variant_index(b)),
        }
    }
}

impl PartialOrd for SqlValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
