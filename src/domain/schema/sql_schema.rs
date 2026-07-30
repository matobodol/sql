use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{
    AutoIncrement, ColumnConstraint, ColumnId, DomainError, Row, SqlType, SqlValue,
    TableConstraint, eval_expr, schema::ColumnDef,
};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Schema {
    columns: Vec<ColumnDef>,
    table_constraints: Vec<TableConstraint>,
}

impl Schema {
    /// Mencari indeks posisi kolom berdasarkan ColumnId (Sangat Cepat O(1)/O(N) tanpa string compare)
    pub fn index_of_id(&self, id: ColumnId) -> Option<usize> {
        self.columns.iter().position(|col| col.id == id)
    }

    /// Mencari referensi ColumnDef berdasarkan ColumnId
    pub fn get_column_by_id(&self, id: ColumnId) -> Option<&ColumnDef> {
        self.columns.iter().find(|col| col.id == id)
    }
    /// Konstruktor utama dengan validasi atomik
    pub fn new(columns: Vec<ColumnDef>) -> Result<Self, DomainError> {
        Self::validate_schema_columns(&columns)?;

        Ok(Self {
            columns,
            table_constraints: Vec::new(),
        })
    }

    /// Helper Internal: Validasi integritas list kolom (Nama Duplikat & Kombinasi Constraint)
    fn validate_schema_columns(columns: &[ColumnDef]) -> Result<(), DomainError> {
        let mut seen_names = HashSet::new();

        for col in columns {
            let col_name_lower = col.name.to_lowercase();

            // 1. Cek Duplikasi Nama Kolom (Case-Insensitive)
            if seen_names.contains(&col_name_lower) {
                return Err(DomainError::EvaluationError(format!(
                    "Duplikat nama kolom '{}' dalam skema tabel",
                    col.name
                )));
            }
            seen_names.insert(col_name_lower);

            // 2. Cek Kombinasi Constraint Kolom
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
                return Err(DomainError::EvaluationError(format!(
                    "Kolom '{}' memiliki lebih dari satu constraint DEFAULT",
                    col.name
                )));
            }

            if has_auto_increment && default_count > 0 {
                return Err(DomainError::EvaluationError(format!(
                    "Kolom '{}' tidak boleh memiliki AutoIncrement dan DEFAULT sekaligus",
                    col.name
                )));
            }
        }

        Ok(())
    }

    /// MULTIPLE ADD COLUMNS (ATOMIK / ALL-OR-NOTHING)
    /// Menambahkan beberapa kolom sekaligus ke skema.
    /// Jika 1 saja kolom invalid, skema asli TIDAK AKAN tersentuh/berubah sama sekali.
    pub fn add_columns(&mut self, new_columns: Vec<ColumnDef>) -> Result<(), DomainError> {
        // 1. Buat klon sementara dari daftar kolom yang ada saat ini (Staging Phase)
        let mut staged_columns = self.columns.clone();

        // 2. Masukkan seluruh kolom baru ke staging list
        staged_columns.extend(new_columns);

        // 3. Validasi SELURUH gabungan kolom di staging list
        // Jika gagal di langkah ini, function langsung return Err()
        // dan `self.columns` asli TIDAK TERSENTUH sama sekali!
        Self::validate_schema_columns(&staged_columns)?;

        // 4. Commit Phase: Jika dan hanya jika validasi 100% lolos, perbarui state asli
        self.columns = staged_columns;
        Ok(())
    }

    pub fn with_table_constraints(
        columns: Vec<ColumnDef>,
        table_constraints: Vec<TableConstraint>,
    ) -> Self {
        Self {
            columns,
            table_constraints,
        }
    }

    /// Mengubah nama tampilan (display name) dari kolom berdasarkan ColumnId
    pub fn rename_column(&mut self, col_id: ColumnId, new_name: &str) -> Result<(), DomainError> {
        if let Some(col) = self.columns.iter_mut().find(|c| c.id == col_id) {
            col.name = new_name.to_string();
            Ok(())
        } else {
            Err(DomainError::EvaluationError(format!(
                "ColumnId '{:?}' tidak ditemukan di Schema",
                col_id
            )))
        }
    }

    /// Accessor read-only untuk daftar kolom
    pub fn columns(&self) -> &[ColumnDef] {
        &self.columns
    }

    /// Mengubah SqlType dari kolom tertentu berdasarkan ColumnId
    pub fn modify_column_type(
        &mut self,
        col_id: ColumnId,
        new_type: SqlType,
    ) -> Result<(), DomainError> {
        let col = self
            .columns
            .iter_mut()
            .find(|c| c.id == col_id)
            .ok_or_else(|| {
                DomainError::EvaluationError(format!(
                    "ColumnId '{:?}' tidak ditemukan pada Schema",
                    col_id
                ))
            })?;

        col.sql_type = new_type;
        Ok(())
    }

    // /// Accessor mutable untuk daftar kolom
    // pub fn columns_mut(&mut self) -> &mut Vec<ColumnDef> {
    //     &mut self.columns
    // }

    pub fn table_constraints(&self) -> &[TableConstraint] {
        &self.table_constraints
    }

    /// Mencari indeks posisi kolom berdasarkan nama
    pub fn index_of(&self, col_name: &str) -> Option<usize> {
        // 1. Coba pencarian persis (exact match atau ignore case)
        if let Some(idx) = self
            .columns
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case(col_name))
        {
            return Some(idx);
        }

        // 2. Jika col_name mengandung format "table.column", ambil bagian nama kolomnya
        if let Some((_table, col)) = col_name.split_once('.') {
            return self
                .columns
                .iter()
                .position(|c| c.name.eq_ignore_ascii_case(col));
        }

        None
    }

    /// Validasi apakah sebuah row sesuai dengan skema ini
    pub fn validate_row(&self, values: &[SqlValue]) -> Result<(), DomainError> {
        if values.len() != self.columns.len() {
            return Err(DomainError::EvaluationError(format!(
                "Jumlah kolom tidak sesuai: mengharapkan {}, ditemukan {}",
                self.columns.len(),
                values.len()
            )));
        }

        let temp_row = Row::new(values.to_vec());

        for (col, val) in self.columns.iter().zip(values.iter()) {
            if val == &SqlValue::Null {
                if !col.is_nullable() {
                    return Err(DomainError::EvaluationError(format!(
                        "Kolom '{}' tidak boleh NULL",
                        col.name
                    )));
                }
            } else if !val.matches_type(&col.sql_type) {
                return Err(DomainError::TypeMismatch {
                    expected: format!("{:?}", col.sql_type),
                    found: format!("{:?}", val),
                });
            }

            // --- VALIDASI COLUMN CHECK CONSTRAINT ---
            for constraint in &col.constraints {
                if let ColumnConstraint::Check(expr) = constraint {
                    let res = eval_expr(expr, self, &temp_row)?;
                    if !res.is_null() && res == SqlValue::Bool(false) {
                        return Err(DomainError::EvaluationError(format!(
                            "Pelanggaran CHECK constraint pada kolom '{}'",
                            col.name
                        )));
                    }
                }
            }
        }

        // --- VALIDASI TABLE CHECK CONSTRAINT ---
        for t_constraint in &self.table_constraints {
            if let TableConstraint::Check(expr) = t_constraint {
                let res = eval_expr(expr, self, &temp_row)?;
                if !res.is_null() && res == SqlValue::Bool(false) {
                    return Err(DomainError::EvaluationError(
                        "Pelanggaran CHECK constraint pada tabel".into(),
                    ));
                }
            }
        }

        Ok(())
    }
}

// validate constraint
impl Schema {
    pub fn validate_column_constraints(columns: &[ColumnDef]) -> Result<(), DomainError> {
        for col in columns {
            let mut default_count = 0;
            let mut has_auto_increment = false;

            for constraint in &col.constraints {
                match constraint {
                    ColumnConstraint::Default(_) => default_count += 1,
                    ColumnConstraint::AutoIncrement(auto_inc) => {
                        if matches!(auto_inc, AutoIncrement::Enabled { .. }) {
                            has_auto_increment = true;
                        }
                    }
                    _ => {}
                }
            }

            // 1. Cek duplikasi constraint DEFAULT
            if default_count > 1 {
                return Err(DomainError::EvaluationError(format!(
                    "Kolom '{}' memiliki lebih dari satu constraint DEFAULT",
                    col.name
                )));
            }

            // 2. Cek konflik AutoIncrement + DEFAULT
            if has_auto_increment && default_count > 0 {
                return Err(DomainError::EvaluationError(format!(
                    "Kolom '{}' tidak boleh memiliki AutoIncrement dan DEFAULT sekaligus",
                    col.name
                )));
            }

            // 3. Cek Tipe Data AutoIncrement
            if has_auto_increment && !matches!(col.sql_type, SqlType::Int) {
                return Err(DomainError::EvaluationError(format!(
                    "AutoIncrement pada kolom '{}' hanya dapat digunakan untuk tipe data Int",
                    col.name
                )));
            }
        }

        Ok(())
    }
}
