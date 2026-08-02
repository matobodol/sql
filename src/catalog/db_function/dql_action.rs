//! Modul aksi eksekusi DQL (Data Query Language) untuk mengeksekusi query `SELECT` dan perintah katalog `SHOW`.

use crate::RowId;
use crate::catalog::database::Database;
use crate::domain::id::ColumnId;
use crate::domain::{ColumnDef, DomainError, Row, Schema, SqlType, SqlValue};
use crate::execution::aggregate::AggregateFunc;
use crate::execution::sort::OrderByExpr;
use crate::expr::Expr;
use crate::planner::PhysicalPlanner;

/// Pernyataan Query `SELECT` (Data Query Language - DQL).
#[derive(Debug, Clone)]
pub struct SelectStmt {
    /// Daftar ekspresi hasil proyeksi (`SELECT col1, expr2, ...`).
    pub projection: Vec<Expr>,
    /// Kondisi predikat penyaringan baris (`WHERE ...`).
    pub selection: Option<Expr>,
    /// Daftar ID kolom untuk kunci pengelompokan (`GROUP BY ...`).
    pub group_by: Vec<ColumnId>,
    /// Daftar fungsi agregasi yang akan dievaluasi (`COUNT`, `SUM`, `AVG`, dll).
    pub aggregates: Vec<AggregateFunc>,
    /// Aturan pengurutan baris keluaran (`ORDER BY ...`).
    pub order_by: Vec<OrderByExpr>,
    /// Batas maksimum jumlah baris keluaran (`LIMIT ...`).
    pub limit: Option<usize>,
    /// Jumlah baris awal yang diabaikan (`OFFSET ...`).
    pub offset: usize,
}

/// Hasil dari eksekusi Query DQL (`SELECT` / `SHOW`).
#[derive(Debug, Clone)]
pub struct DqlResult {
    /// Skema kolom dari tabel hasil query.
    pub schema: Schema,
    /// Daftar baris data yang dihasilkan.
    pub rows: Vec<Row>,
}

/// Jenis kueri inspeksi katalog (`SHOW TABLES`, dll).
#[derive(Debug, Clone)]
pub enum Show<'a> {
    /// Menampilkan seluruh nama tabel dalam basis data.
    Tables,
    /// Menampilkan nama tabel yang cocok dengan pola `LIKE`.
    TablesLike(&'a str),
}

/// Menjalankan query `SELECT` menggunakan `PhysicalPlanner` dan mengeksekusinya secara *Lazy Volcano Execution*.
pub fn execute_select(
    db: &Database,
    table_name: &str,
    stmt: SelectStmt,
) -> Result<DqlResult, DomainError> {
    // 1. Serahkan delegasi penyusunan execution tree ke PhysicalPlanner
    let mut plan = PhysicalPlanner::build_plan(db, table_name, &stmt)?;

    // 2. Eksekusi Pipeline secara Lazy (Pull Data)
    let final_schema = plan.schema().clone();
    let mut result_rows = Vec::new();

    while let Some(row) = plan.next()? {
        result_rows.push(row);
    }

    Ok(DqlResult {
        schema: final_schema,
        rows: result_rows,
    })
}

/// Mengeksekusi perintah kueri katalog `SHOW`.
pub fn execute_show(database: &Database, show: Show) -> Result<DqlResult, DomainError> {
    match show {
        Show::Tables => show_tables(database),
        Show::TablesLike(pattern) => show_tables_like(database, pattern),
    }
}

// =========================================================================
// CATALOG & SYSTEM METADATA QUERIES
// =========================================================================

fn show_tables(database: &Database) -> Result<DqlResult, DomainError> {
    let col_def = ColumnDef::new(ColumnId(1), "table_name", SqlType::Text);
    let schema = Schema::new(vec![col_def])?;

    let table_names = database.list_tables();
    let mut rows = Vec::with_capacity(table_names.len());
    for (idx, name) in table_names.into_iter().enumerate() {
        let row_id = RowId((idx + 1) as u64);
        let values = vec![SqlValue::Text(name)];
        rows.push(Row::with_id(row_id, values));
    }

    Ok(DqlResult { schema, rows })
}

fn show_tables_like(database: &Database, pattern: &str) -> Result<DqlResult, DomainError> {
    let col_def = ColumnDef::new(ColumnId(1), "table_name", SqlType::Text);
    let schema = Schema::new(vec![col_def])?;

    let pattern_val = SqlValue::Text(pattern.to_string());
    let mut matching_tables = Vec::new();

    for name in database.list_tables() {
        let name_val = SqlValue::Text(name.clone());

        if name_val.like(&pattern_val)?.is_true() {
            matching_tables.push(name);
        }
    }

    let rows = matching_tables
        .into_iter()
        .enumerate()
        .map(|(idx, name)| {
            let row_id = RowId((idx + 1) as u64);
            Row::with_id(row_id, vec![SqlValue::Text(name)])
        })
        .collect();

    Ok(DqlResult { schema, rows })
}
