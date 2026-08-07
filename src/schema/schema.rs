use std::collections::HashSet;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{
    ColumnConstraint, DomainError, Expr, Row, SqlType, SqlValue, TableConstraint, eval_expr,
    id::{ColumnId, RowId},
    schema::Column,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AutoIncrement {
    /// Auto increment biasa (increment +1)
    Enabled {
        start: i64,
        step: i64,
    },
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Schema {
    columns: Vec<Column>,
    table_constraints: Vec<TableConstraint>,
}

impl Schema {
    // =========================================================================
    // KONSTRUKTOR & STAGING VALIDATOR
    // =========================================================================

    /// Konstruktor utama dengan validasi atomik
    pub fn new(columns: Vec<Column>) -> Result<Self, DomainError> {
        Self::validate_schema_columns(&columns)?;

        Ok(Self {
            columns,
            table_constraints: Vec::new(),
        })
    }

    pub fn with_table_constraints(
        columns: Vec<Column>,
        table_constraints: Vec<TableConstraint>,
    ) -> Result<Self, DomainError> {
        Self::validate_schema_columns(&columns)?;

        Ok(Self {
            columns,
            table_constraints,
        })
    }

    /// Single Source of Truth Validator untuk seluruh daftar kolom
    pub fn validate_schema_columns(columns: &[Column]) -> Result<(), DomainError> {
        let mut seen_names = HashSet::with_capacity(columns.len());

        for col in columns {
            let col_name_lower = col.name.to_lowercase();

            // 1. Validasi duplikasi varian Enum
            col.sql_type.validate_enum_variants()?;

            // 2. Cek Duplikasi Nama Kolom (Case-Insensitive)
            if !seen_names.insert(col_name_lower) {
                return Err(DomainError::eval_error(format!(
                    "Duplikat nama kolom '{}' dalam skema tabel",
                    col.name
                )));
            }

            // 3. Cek Kombinasi Constraint Kolom
            let mut default_count = 0;
            let mut has_auto_increment = false;

            for constraint in &col.constraints {
                match constraint {
                    ColumnConstraint::Default(_) => default_count += 1,
                    ColumnConstraint::AutoIncrement(AutoIncrement::Enabled { .. }) => {
                        has_auto_increment = true;
                    }
                    _ => {}
                }
            }

            if default_count > 1 {
                return Err(DomainError::eval_error(format!(
                    "Kolom '{}' memiliki lebih dari satu constraint DEFAULT",
                    col.name
                )));
            }

            if has_auto_increment && default_count > 0 {
                return Err(DomainError::eval_error(format!(
                    "Kolom '{}' tidak boleh memiliki AutoIncrement dan DEFAULT sekaligus",
                    col.name
                )));
            }

            // 4. Cek Tipe Data AutoIncrement (Wajib Int)
            if has_auto_increment && !matches!(col.sql_type, SqlType::Int) {
                return Err(DomainError::eval_error(format!(
                    "AutoIncrement pada kolom '{}' hanya dapat digunakan untuk tipe data Int",
                    col.name
                )));
            }
        }

        Ok(())
    }

    // =========================================================================
    // MUTATION METHODS (DENGAN RE-VALIDATION ATOMIK)
    // =========================================================================

    /// MULTIPLE ADD COLUMNS (ATOMIK)
    pub fn add_columns(&mut self, new_columns: Vec<Column>) -> Result<(), DomainError> {
        let mut staged_columns = self.columns.clone();
        staged_columns.extend(new_columns);

        Self::validate_schema_columns(&staged_columns)?;
        self.columns = staged_columns;
        Ok(())
    }

    /// Mengubah SqlType dari kolom tertentu secara aman
    pub fn modify_column_type(
        &mut self,
        col_id: ColumnId,
        new_type: SqlType,
    ) -> Result<(), DomainError> {
        let mut staged_columns = self.columns.clone();
        let col = staged_columns
            .iter_mut()
            .find(|c| c.id == col_id)
            .ok_or_else(|| {
                DomainError::eval_error(format!(
                    "ColumnId '{:?}' tidak ditemukan pada Schema",
                    col_id
                ))
            })?;

        col.sql_type = new_type;

        // Re-validate seluruh skema (Cek enum variants & AutoIncrement compatibility)
        Self::validate_schema_columns(&staged_columns)?;
        self.columns = staged_columns;
        Ok(())
    }

    /// Menambahkan constraint baru ke kolom secara aman
    pub fn add_column_constraint(
        &mut self,
        col_id: ColumnId,
        constraint: ColumnConstraint,
    ) -> Result<(), DomainError> {
        let mut staged_columns = self.columns.clone();
        let col = staged_columns
            .iter_mut()
            .find(|c| c.id == col_id)
            .ok_or_else(|| DomainError::eval_error("Kolom tidak ditemukan pada Schema"))?;

        col.constraints.push(constraint);

        // Re-validate seluruh skema (Cek konflik DEFAULT & AutoIncrement)
        Self::validate_schema_columns(&staged_columns)?;
        self.columns = staged_columns;
        Ok(())
    }

    /// Mengubah nama kolom
    pub fn rename_column(&mut self, col_id: ColumnId, new_name: &str) -> Result<(), DomainError> {
        let mut staged_columns = self.columns.clone();
        let col = staged_columns
            .iter_mut()
            .find(|c| c.id == col_id)
            .ok_or_else(|| {
                DomainError::eval_error(format!(
                    "ColumnId '{:?}' tidak ditemukan di Schema",
                    col_id
                ))
            })?;

        col.name = new_name.to_string();

        // Re-validate (Cek duplikasi nama)
        Self::validate_schema_columns(&staged_columns)?;
        self.columns = staged_columns;
        Ok(())
    }

    /// Menghapus constraint dari kolom
    pub fn drop_column_constraint(
        &mut self,
        col_id: ColumnId,
        constraint: &ColumnConstraint,
    ) -> Result<(), DomainError> {
        let col = self
            .columns
            .iter_mut()
            .find(|c| c.id == col_id)
            .ok_or_else(|| DomainError::eval_error("Kolom tidak ditemukan pada Schema"))?;

        let initial_len = col.constraints.len();
        col.constraints.retain(|c| c != constraint);

        if col.constraints.len() == initial_len {
            return Err(DomainError::eval_error(format!(
                "Constraint '{:?}' tidak ditemukan pada kolom",
                constraint
            )));
        }

        Ok(())
    }

    /// Mengubah atau menghapus nilai default pada ColumnDef
    pub fn set_column_default(
        &mut self,
        col_id: ColumnId,
        default_val: Option<SqlValue>,
    ) -> Result<(), DomainError> {
        let mut staged_columns = self.columns.clone();
        let col = staged_columns
            .iter_mut()
            .find(|c| c.id == col_id)
            .ok_or_else(|| DomainError::eval_error("Kolom tidak ditemukan pada Schema"))?;

        col.constraints
            .retain(|c| !matches!(c, ColumnConstraint::Default(_)));

        if let Some(val) = default_val {
            col.constraints.push(ColumnConstraint::Default(val));
        }

        Self::validate_schema_columns(&staged_columns)?;
        self.columns = staged_columns;
        Ok(())
    }

    // =========================================================================
    // READ HELPERS & ROW VALIDATION
    // =========================================================================

    #[inline]
    pub fn index_of_id(&self, id: ColumnId) -> Option<usize> {
        self.columns.iter().position(|col| col.id == id)
    }

    #[inline]
    pub fn index_of_name(&self, name: &str) -> Option<usize> {
        self.columns
            .iter()
            .position(|col| col.name.eq_ignore_ascii_case(name))
    }

    /// Alias pendukung untuk kompatibilitas pencarian indeks berbasis nama
    #[inline]
    pub fn get_column_index_by_name(&self, name: &str) -> Option<usize> {
        self.index(name)
    }

    #[inline]
    pub fn get_column_by_id(&self, id: ColumnId) -> Option<&Column> {
        self.columns.iter().find(|col| col.id == id)
    }

    #[inline]
    pub fn get_column_by_name(&self, name: &str) -> Option<&Column> {
        self.index(name).map(|idx| &self.columns[idx])
    }

    #[inline]
    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    #[inline]
    pub fn table_constraints(&self) -> &[TableConstraint] {
        &self.table_constraints
    }

    /// Pencarian indeks fleksibel (Mendukung nama kolom biasa & format qualified `table.column`)
    pub fn index(&self, col_name: &str) -> Option<usize> {
        if let Some(idx) = self
            .columns
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case(col_name))
        {
            return Some(idx);
        }

        if let Some((_table, col)) = col_name.split_once('.') {
            return self
                .columns
                .iter()
                .position(|c| c.name.eq_ignore_ascii_case(col));
        }

        None
    }

    /// Deprecated fallback alias untuk `index`
    #[inline]
    pub fn index_of(&self, col_name: &str) -> Option<usize> {
        self.index(col_name)
    }

    pub fn validate_row(&self, values: &[SqlValue]) -> Result<(), DomainError> {
        if values.len() != self.columns.len() {
            return Err(DomainError::eval_error(format!(
                "Jumlah kolom tidak sesuai: mengharapkan {}, ditemukan {}",
                self.columns.len(),
                values.len()
            )));
        }

        // RowId dummy (0) khusus untuk evaluasi CHECK constraint
        let temp_row = Row::with_id(RowId::from(0u64), values.to_vec());

        for (col, val) in self.columns.iter().zip(values.iter()) {
            if val.is_null() {
                if !col.is_nullable() {
                    return Err(DomainError::eval_error(format!(
                        "Kolom '{}' tidak boleh NULL",
                        col.name
                    )));
                }
            } else if !val.matches_type(&col.sql_type) {
                return Err(DomainError::TypeMismatch {
                    expected: Arc::from(format!("{:?}", col.sql_type).as_str()),
                    found: Arc::from(format!("{:?}", val).as_str()),
                });
            }

            for constraint in &col.constraints {
                if let ColumnConstraint::Check(expr) = constraint {
                    // Perbaikan: Lakukan bind_expr_columns sebelum memanggil eval_expr(bound_expr, row)
                    let bound_expr = bind_expr_columns(expr, self)?;
                    let res = eval_expr(&bound_expr, &temp_row)?;

                    if !res.is_null() && res.as_ref() == &SqlValue::Bool(false) {
                        return Err(DomainError::eval_error(format!(
                            "Pelanggaran CHECK constraint pada kolom '{}'",
                            col.name
                        )));
                    }
                }
            }
        }

        for t_constraint in &self.table_constraints {
            if let TableConstraint::Check(expr) = t_constraint {
                // Perbaikan: Pre-bind ekspresi tingkat tabel ke ColumnIndex
                let bound_expr = bind_expr_columns(expr, self)?;
                let res = eval_expr(&bound_expr, &temp_row)?;

                if !res.is_null() && res.as_ref() == &SqlValue::Bool(false) {
                    return Err(DomainError::eval_error(
                        "Pelanggaran CHECK constraint pada tabel",
                    ));
                }
            }
        }

        Ok(())
    }
}

/// Pre-binding helper: Mengonversi `Expr::Column(name)` ke `Expr::ColumnIndex(offset)` O(1)
/// menggunakan pementa-an indeks dari `Schema`.
pub fn bind_expr_columns(expr: &Expr, schema: &Schema) -> Result<Expr, DomainError> {
    match expr {
        Expr::Column(name) => {
            let idx = schema
                .index(name)
                .ok_or_else(|| DomainError::ColumnNotFound(Arc::from(name.as_str())))?;

            Ok(Expr::ColumnIndex(idx))
        }
        Expr::Binary { left, op, right } => {
            let bound_left = bind_expr_columns(left, schema)?;
            let bound_right = bind_expr_columns(right, schema)?;
            Ok(Expr::Binary {
                left: Box::new(bound_left),
                op: *op,
                right: Box::new(bound_right),
            })
        }
        Expr::Not(inner) => Ok(Expr::Not(Box::new(bind_expr_columns(inner, schema)?))),
        Expr::IsNull(inner) => Ok(Expr::IsNull(Box::new(bind_expr_columns(inner, schema)?))),
        Expr::IsNotNull(inner) => Ok(Expr::IsNotNull(Box::new(bind_expr_columns(inner, schema)?))),
        Expr::InList { expr, list } => {
            let bound_target = bind_expr_columns(expr, schema)?;
            let bound_list = list
                .iter()
                .map(|item| bind_expr_columns(item, schema))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Expr::InList {
                expr: Box::new(bound_target),
                list: bound_list,
            })
        }
        // Varian literal atau yang sudah ter-bind (ColumnIndex, Literal, dll.)
        other => Ok(other.clone()),
    }
}
