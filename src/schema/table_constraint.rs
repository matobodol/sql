use serde::{Deserialize, Serialize};

use crate::Expr;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TableConstraint {
    /// Primary Key komposit (gabungan beberapa kolom, misal: [order_id, item_id])
    PrimaryKey(Vec<String>),
    /// Unique Key komposit (kombinasi beberapa kolom harus unik)
    Unique(Vec<String>),
    /// Foreign Key untuk integritas referensial ke tabel lain
    ForeignKey {
        columns: Vec<String>,
        foreign_table: String,
        foreign_columns: Vec<String>,
    },
    /// Pengecekan ekspresi tingkat tabel (misal: end_date >= start_date)
    Check(Expr),
}
