use super::sql_type::{SqlBool, SqlValue};

impl SqlValue {
    /// Membandingkan kesamaan dua `SqlValue` (Logika 3VL SQL)
    pub fn eq(&self, other: &Self) -> SqlBool {
        match (self, other) {
            (SqlValue::Null, _) | (_, SqlValue::Null) => SqlBool::Unknown,
            (SqlValue::Int(a), SqlValue::Int(b)) => {
                if a == b {
                    SqlBool::True
                } else {
                    SqlBool::False
                }
            }
            (SqlValue::Float(a), SqlValue::Float(b)) => {
                if a == b {
                    SqlBool::True
                } else {
                    SqlBool::False
                }
            }
            (SqlValue::Text(a), SqlValue::Text(b)) => {
                if a == b {
                    SqlBool::True
                } else {
                    SqlBool::False
                }
            }
            (SqlValue::Bool(a), SqlValue::Bool(b)) => {
                if a == b {
                    SqlBool::True
                } else {
                    SqlBool::False
                }
            }
            (SqlValue::Timestamp(a), SqlValue::Timestamp(b)) => {
                if a == b {
                    SqlBool::True
                } else {
                    SqlBool::False
                }
            }
            (SqlValue::Date(a), SqlValue::Date(b)) => {
                if a == b {
                    SqlBool::True
                } else {
                    SqlBool::False
                }
            }
            (SqlValue::Time(a), SqlValue::Time(b)) => {
                if a == b {
                    SqlBool::True
                } else {
                    SqlBool::False
                }
            }
            (SqlValue::Bytes(a), SqlValue::Bytes(b)) => {
                if a == b {
                    SqlBool::True
                } else {
                    SqlBool::False
                }
            }
            // Coercion sederhana jika int dibandingkan float
            (SqlValue::Int(a), SqlValue::Float(b)) => {
                if (*a as f64) == *b {
                    SqlBool::True
                } else {
                    SqlBool::False
                }
            }
            (SqlValue::Float(a), SqlValue::Int(b)) => {
                if *a == (*b as f64) {
                    SqlBool::True
                } else {
                    SqlBool::False
                }
            }
            _ => SqlBool::False,
        }
    }

    /// Membandingkan apakah `self` lebih besar dari `other` (3VL)
    pub fn gt(&self, other: &Self) -> SqlBool {
        match (self, other) {
            (SqlValue::Null, _) | (_, SqlValue::Null) => SqlBool::Unknown,
            (SqlValue::Int(a), SqlValue::Int(b)) => {
                if a > b {
                    SqlBool::True
                } else {
                    SqlBool::False
                }
            }
            (SqlValue::Float(a), SqlValue::Float(b)) => {
                if a > b {
                    SqlBool::True
                } else {
                    SqlBool::False
                }
            }
            (SqlValue::Text(a), SqlValue::Text(b)) => {
                if a > b {
                    SqlBool::True
                } else {
                    SqlBool::False
                }
            }
            (SqlValue::Timestamp(a), SqlValue::Timestamp(b)) => {
                if a > b {
                    SqlBool::True
                } else {
                    SqlBool::False
                }
            }
            (SqlValue::Date(a), SqlValue::Date(b)) => {
                if a > b {
                    SqlBool::True
                } else {
                    SqlBool::False
                }
            }
            (SqlValue::Time(a), SqlValue::Time(b)) => {
                if a > b {
                    SqlBool::True
                } else {
                    SqlBool::False
                }
            }
            _ => SqlBool::False,
        }
    }
}
