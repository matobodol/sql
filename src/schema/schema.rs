use std::collections::HashSet;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{
    ColumnConstraint, ColumnId, DataType, DomainError, Expr, Increment, TableConstraint, ValueType,
    schema::Column, validator::validate_enum_variants,
};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Schema {
    columns: Vec<Column>,
    table_constraints: Vec<TableConstraint>,
    /// Cache ekspresi CHECK yang ter-bind ke indeks kolom untuk evaluasi O(1)
    #[serde(skip)]
    bound_column_checks: Vec<(usize, Expr)>,
    #[serde(skip)]
    bound_table_checks: Vec<Expr>,
    #[serde(skip)]
    has_check_constraints: bool,
}

impl Schema {
    /// Menyisipkan kolom baru pada posisi indeks tertentu di skema
    /// dan mengompilasi ulang evaluasi CHECK constraint.
    pub fn insert_column(&mut self, index: usize, col: Column) -> Result<(), DomainError> {
        if index > self.columns.len() {
            return Err(DomainError::eval_error(format!(
                "Indeks penyisipan kolom {index} melebihi jumlah kolom {}",
                self.columns.len()
            )));
        }

        let mut staged_columns = self.columns.clone();
        staged_columns.insert(index, col);

        Self::validate_schema_columns(&staged_columns)?;
        self.columns = staged_columns;
        self.compile_check_constraints()?;
        Ok(())
    }

    pub fn columns_mut(&mut self) -> &mut Vec<Column> {
        &mut self.columns
    }
    /// Menghapus kolom dari skema berdasarkan `ColumnId`.
    pub fn remove_column(&mut self, col_id: ColumnId) -> Result<Column, DomainError> {
        let position = self
            .columns
            .iter()
            .position(|col| col.id == col_id)
            .ok_or_else(|| {
                DomainError::eval_error(format!(
                    "ColumnId {:?} tidak ditemukan di dalam skema",
                    col_id
                ))
            })?;

        Ok(self.columns.remove(position))
    }
}

impl Schema {
    // =========================================================================
    // KONSTRUKTOR & STAGING VALIDATOR
    // =========================================================================

    pub fn new(columns: Vec<Column>) -> Result<Self, DomainError> {
        Self::with_table_constraints(columns, Vec::new())
    }

    pub fn with_table_constraints(
        columns: Vec<Column>,
        table_constraints: Vec<TableConstraint>,
    ) -> Result<Self, DomainError> {
        Self::validate_schema_columns(&columns)?;

        let mut schema = Self {
            columns,
            table_constraints,
            bound_column_checks: Vec::new(),
            bound_table_checks: Vec::new(),
            has_check_constraints: false,
        };

        schema.compile_check_constraints()?;
        Ok(schema)
    }

    /// Mengompilasi dan mengikat seluruh ekspresi CHECK constraint ke ColumnIndex
    fn compile_check_constraints(&mut self) -> Result<(), DomainError> {
        self.bound_column_checks.clear();
        self.bound_table_checks.clear();

        for (col_idx, col) in self.columns.iter().enumerate() {
            for constraint in &col.constraints {
                if let ColumnConstraint::Check(expr) = constraint {
                    let bound_expr = bind_expr_columns(expr, self)?;
                    self.bound_column_checks.push((col_idx, bound_expr));
                }
            }
        }

        for t_constraint in &self.table_constraints {
            if let TableConstraint::Check(expr) = t_constraint {
                let bound_expr = bind_expr_columns(expr, self)?;
                self.bound_table_checks.push(bound_expr);
            }
        }

        self.has_check_constraints =
            !self.bound_column_checks.is_empty() || !self.bound_table_checks.is_empty();

        Ok(())
    }

    pub fn validate_schema_columns(columns: &[Column]) -> Result<(), DomainError> {
        let mut seen_names = HashSet::with_capacity(columns.len());

        for col in columns {
            let col_name_lower = col.name.clone();

            validate_enum_variants(&col.sql_type)?;

            if !seen_names.insert(col_name_lower) {
                return Err(DomainError::eval_error(format!(
                    "Duplikat nama kolom '{}' dalam skema tabel",
                    col.name
                )));
            }

            let mut default_count = 0;
            let mut has_auto_increment = false;

            for constraint in &col.constraints {
                match constraint {
                    ColumnConstraint::Default(_) => default_count += 1,
                    ColumnConstraint::Auto(Increment::Enabled { .. }) => {
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

            if has_auto_increment && !matches!(col.sql_type, DataType::Int) {
                return Err(DomainError::eval_error(format!(
                    "AutoIncrement pada kolom '{}' hanya dapat digunakan untuk tipe data Int",
                    col.name
                )));
            }
        }

        Ok(())
    }

    // =========================================================================
    // MUTATION METHODS (DENGAN COMPILATION ATOMIK)
    // =========================================================================

    pub fn add_columns(&mut self, new_columns: Vec<Column>) -> Result<(), DomainError> {
        let mut staged_columns = self.columns.clone();
        staged_columns.extend(new_columns);

        Self::validate_schema_columns(&staged_columns)?;
        self.columns = staged_columns;
        self.compile_check_constraints()?;
        Ok(())
    }

    pub fn modify_column_type(
        &mut self,
        col_id: ColumnId,
        new_type: DataType,
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

        Self::validate_schema_columns(&staged_columns)?;
        self.columns = staged_columns;
        self.compile_check_constraints()?;
        Ok(())
    }

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

        Self::validate_schema_columns(&staged_columns)?;
        self.columns = staged_columns;
        self.compile_check_constraints()?;
        Ok(())
    }

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

        Self::validate_schema_columns(&staged_columns)?;
        self.columns = staged_columns;
        self.compile_check_constraints()?;
        Ok(())
    }

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

        self.compile_check_constraints()?;
        Ok(())
    }

    pub fn set_column_default(
        &mut self,
        col_id: ColumnId,
        default_val: Option<ValueType>,
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
        self.compile_check_constraints()?;
        Ok(())
    }

    // =========================================================================
    // READ HELPERS & OPTIMIZED ROW VALIDATION
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

    #[inline]
    pub fn has_check_constraints(&self) -> bool {
        self.has_check_constraints
    }

    #[inline]
    pub fn bound_column_checks(&self) -> &[(usize, Expr)] {
        &self.bound_column_checks
    }

    #[inline]
    pub fn bound_table_checks(&self) -> &[Expr] {
        &self.bound_table_checks
    }

    #[inline]
    pub fn index_of(&self, col_name: &str) -> Option<usize> {
        self.index(col_name)
    }

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
}

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
        other => Ok(other.clone()),
    }
}
