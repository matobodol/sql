use std::ops::Not;

use ordered_float::OrderedFloat;

use crate::{SqlBool, SqlValue};

impl SqlValue {
    /// EQUAL (==)
    pub fn eq(&self, other: &Self) -> SqlBool {
        match (self, other) {
            (SqlValue::Null, _) | (_, SqlValue::Null) => SqlBool::Unknown,
            (SqlValue::Int(a), SqlValue::Int(b)) => (a == b).into(),
            (SqlValue::Text(a), SqlValue::Text(b)) => (a == b).into(),
            (SqlValue::Bool(a), SqlValue::Bool(b)) => (a == b).into(),
            (SqlValue::Timestamp(a), SqlValue::Timestamp(b)) => (a == b).into(),
            (SqlValue::Date(a), SqlValue::Date(b)) => (a == b).into(),
            (SqlValue::Time(a), SqlValue::Time(b)) => (a == b).into(),
            (SqlValue::Bytes(a), SqlValue::Bytes(b)) => (a == b).into(),

            // --- FLOAT COMPARISON
            (SqlValue::Float(a), SqlValue::Float(b)) => (a == b).into(),
            (SqlValue::Int(a), SqlValue::Float(b)) => (OrderedFloat(*a as f64) == *b).into(),
            (SqlValue::Float(a), SqlValue::Int(b)) => (*a == OrderedFloat(*b as f64)).into(),
            _ => false.into(),
        }
    }

    /// GREATER THAN (>)
    pub fn gt(&self, other: &Self) -> SqlBool {
        match (self, other) {
            (SqlValue::Null, _) | (_, SqlValue::Null) => SqlBool::Unknown,
            (SqlValue::Int(a), SqlValue::Int(b)) => (a > b).into(),
            (SqlValue::Float(a), SqlValue::Float(b)) => (a > b).into(),
            (SqlValue::Text(a), SqlValue::Text(b)) => (a > b).into(),
            (SqlValue::Timestamp(a), SqlValue::Timestamp(b)) => (a > b).into(),
            (SqlValue::Date(a), SqlValue::Date(b)) => (a > b).into(),
            (SqlValue::Time(a), SqlValue::Time(b)) => (a > b).into(),
            _ => false.into(),
        }
    }

    /// LESS THAN (<)
    pub fn lt(&self, other: &Self) -> SqlBool {
        match (self, other) {
            (SqlValue::Null, _) | (_, SqlValue::Null) => SqlBool::Unknown,
            (SqlValue::Int(a), SqlValue::Int(b)) => (a < b).into(),
            (SqlValue::Float(a), SqlValue::Float(b)) => (a < b).into(),
            (SqlValue::Text(a), SqlValue::Text(b)) => (a < b).into(),
            (SqlValue::Timestamp(a), SqlValue::Timestamp(b)) => (a < b).into(),
            (SqlValue::Date(a), SqlValue::Date(b)) => (a < b).into(),
            (SqlValue::Time(a), SqlValue::Time(b)) => (a < b).into(),
            _ => false.into(),
        }
    }

    /// GREATER THAN OR EQUAL (>=)
    pub fn gteq(&self, other: &Self) -> SqlBool {
        match (self, other) {
            (SqlValue::Null, _) | (_, SqlValue::Null) => SqlBool::Unknown,
            (SqlValue::Int(a), SqlValue::Int(b)) => (a >= b).into(),
            (SqlValue::Float(a), SqlValue::Float(b)) => (a >= b).into(),
            (SqlValue::Text(a), SqlValue::Text(b)) => (a >= b).into(),
            (SqlValue::Timestamp(a), SqlValue::Timestamp(b)) => (a >= b).into(),
            (SqlValue::Date(a), SqlValue::Date(b)) => (a >= b).into(),
            (SqlValue::Time(a), SqlValue::Time(b)) => (a >= b).into(),
            _ => false.into(),
        }
    }

    /// LESS THAN OR EQUAL (<=)
    pub fn lteq(&self, other: &Self) -> SqlBool {
        match (self, other) {
            (SqlValue::Null, _) | (_, SqlValue::Null) => SqlBool::Unknown,
            (SqlValue::Int(a), SqlValue::Int(b)) => (a <= b).into(),
            (SqlValue::Float(a), SqlValue::Float(b)) => (a <= b).into(),
            (SqlValue::Text(a), SqlValue::Text(b)) => (a <= b).into(),
            (SqlValue::Timestamp(a), SqlValue::Timestamp(b)) => (a <= b).into(),
            (SqlValue::Date(a), SqlValue::Date(b)) => (a <= b).into(),
            (SqlValue::Time(a), SqlValue::Time(b)) => (a <= b).into(),
            _ => false.into(),
        }
    }

    // Serahkan logika sepenuhnya ke SqlBool agar 3VL berjalan benar
    /// OPERATOR LOGIKA AND
    pub fn and(&self, other: &Self) -> SqlBool {
        SqlBool::from(self).and(SqlBool::from(other))
    }
    /// OPERATOR LOGIKA OR
    pub fn or(&self, other: &Self) -> SqlBool {
        SqlBool::from(self).or(SqlBool::from(other))
    }
    /// OPERATOR LOGIKA NOT
    pub fn noteq(&self, other: &Self) -> SqlBool {
        self.eq(other).not()
    }
}
