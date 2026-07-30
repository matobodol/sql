use crate::catalog::Database;
use crate::domain::{ColumnConstraint, ColumnDef, DomainError, Schema, SqlType, SqlValue};

/// Representasi Aksi ALTER TABLE berstandar ANSI SQL
#[derive(Debug, Clone, PartialEq)]
pub enum AlterTableAction {
    AddColumn {
        name: String,
        sql_type: SqlType,
        constraints: Vec<ColumnConstraint>,
    },
    DropColumn {
        name: String,
    },
    RenameColumn {
        old_name: String,
        new_name: String,
    },
    RenameTable {
        new_name: String,
    },
    ModifyColumnType {
        name: String,
        new_type: SqlType,
    },
    AddConstraint {
        col_name: String,
        constraint: ColumnConstraint,
    },
    DropConstraint {
        col_name: String,
        constraint: ColumnConstraint,
    },
    SetDefault {
        col_name: String,
        default_val: Option<SqlValue>,
    },
}

impl Database {
    /// Eksekusi rantai aksi ALTER TABLE (Multi-action support)
    pub fn alter_table(
        &mut self,
        table_name: &str,
        actions: Vec<AlterTableAction>,
    ) -> Result<(), DomainError> {
        // Cek awal keberadaan tabel
        if self.get_table(table_name).is_err() {
            return Err(DomainError::TableNotFound(table_name.to_string()));
        }

        let mut current_table_name = table_name.to_string();

        for action in actions {
            match action {
                AlterTableAction::AddColumn {
                    name,
                    sql_type,
                    constraints,
                } => {
                    self.execute_add_column(&current_table_name, &name, sql_type, constraints)?;
                }
                AlterTableAction::DropColumn { name } => {
                    self.execute_drop_column(&current_table_name, &name)?;
                }
                AlterTableAction::RenameColumn { old_name, new_name } => {
                    self.execute_rename_column(&current_table_name, &old_name, &new_name)?;
                }
                AlterTableAction::RenameTable { new_name } => {
                    self.execute_rename_table(&current_table_name, &new_name)?;
                    current_table_name = new_name; // Melacak nama baru jika ada aksi beruntun
                }
                AlterTableAction::ModifyColumnType { name, new_type } => {
                    self.execute_modify_column_type(&current_table_name, &name, new_type)?;
                }
                _ => (),
            }
        }

        Ok(())
    }

    // --- EXECUTION HELPERS ---

    fn execute_add_column(
        &mut self,
        table_name: &str,
        col_name: &str,
        sql_type: SqlType,
        constraints: Vec<ColumnConstraint>,
    ) -> Result<(), DomainError> {
        // 1. Daftarkan nama kolom ke SymbolRegistry untuk alokasi ColumnId
        let col_id = self.registry.register_column(col_name);

        // 2. Buat ColumnDef & ekstrak default value
        let new_col_def = ColumnDef::with_constraints(col_id, col_name, sql_type, constraints);
        let default_val = new_col_def
            .default_value()
            .cloned()
            .unwrap_or(SqlValue::Null);

        // 3. Ambil mutable table & buat Schema baru
        let table = self.get_table_mut(table_name)?;
        let mut new_columns = table.schema().columns().to_vec();
        new_columns.push(new_col_def);

        // Validasi skema (memastikan tidak ada duplikasi / konflik constraint)
        let new_schema = Schema::new(new_columns)?;
        *table.schema_mut() = new_schema;

        // 4. Backfill seluruh row fisik eksisting
        for row in table.rows_mut() {
            row.push(default_val.clone());
        }

        Ok(())
    }

    fn execute_drop_column(&mut self, table_name: &str, col_name: &str) -> Result<(), DomainError> {
        let col_id = self.registry.get_column_id(col_name).ok_or_else(|| {
            DomainError::EvaluationError(format!("Kolom '{col_name}' tidak ditemukan"))
        })?;

        let table = self.get_table_mut(table_name)?;

        // 1. Cari indeks posisi kolom di Schema
        let col_idx = table
            .schema()
            .columns()
            .iter()
            .position(|c| c.id == col_id)
            .ok_or_else(|| {
                DomainError::EvaluationError(format!("Kolom '{col_name}' tidak ada di tabel"))
            })?;

        // 2. Buat skema baru tanpa kolom tersebut
        let mut new_columns = table.schema().columns().to_vec();
        new_columns.remove(col_idx);
        let new_schema = Schema::new(new_columns)?;
        *table.schema_mut() = new_schema;

        // 3. Hapus nilai pada indeks tersebut di setiap Row menggunakan method `remove`
        for row in table.rows_mut() {
            row.remove(col_idx);
        }

        Ok(())
    }

    fn execute_rename_column(
        &mut self,
        table_name: &str,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), DomainError> {
        let col_id = self.registry.get_column_id(old_name).ok_or_else(|| {
            DomainError::EvaluationError(format!("Kolom '{old_name}' tidak ditemukan"))
        })?;

        // 1. Rename di Registry (O(1))
        self.registry.rename_column(old_name, new_name)?;

        // 2. Rename display name di Schema
        let table = self.get_table_mut(table_name)?;
        table.schema_mut().rename_column(col_id, new_name)?;

        Ok(())
    }

    fn execute_rename_table(&mut self, old_name: &str, new_name: &str) -> Result<(), DomainError> {
        let table_id = self
            .registry
            .get_table_id(old_name)
            .ok_or_else(|| DomainError::TableNotFound(old_name.to_string()))?;

        if self.registry.get_table_id(new_name).is_some() {
            return Err(DomainError::TableAlreadyExists(new_name.to_string()));
        }

        // 1. Update nama tabel di Registry
        // (Pastikan method rename_table ada di SymbolRegistry, atau re-register)
        // 2. Update nama internal di struct Table
        let table = self.tables.get_mut(&table_id).unwrap();
        table.set_name(new_name);

        Ok(())
    }

    fn execute_modify_column_type(
        &mut self,
        table_name: &str,
        col_name: &str,
        new_type: SqlType,
    ) -> Result<(), DomainError> {
        let col_id = self.registry.get_column_id(col_name).ok_or_else(|| {
            DomainError::EvaluationError(format!("Kolom '{col_name}' tidak ditemukan"))
        })?;

        let table = self.get_table_mut(table_name)?;

        // 1. Cari indeks kolom di skema saat ini
        let col_idx = table
            .schema()
            .columns()
            .iter()
            .position(|c| c.id == col_id)
            .ok_or_else(|| {
                DomainError::EvaluationError(format!("Kolom '{col_name}' tidak ada di tabel"))
            })?;

        // 2. Tahap Validasi & Casting seluruh Row (Staging)
        // Kumpulkan nilai baru ke Vec sementara. Jika ada 1 nilai gagal, transaksi batal sebelum skema tersentuh!
        let mut new_values = Vec::with_capacity(table.rows().len());
        for row in table.rows() {
            let current_val = &row[col_idx];
            let casted_val = current_val.try_cast_to(&new_type)?;
            new_values.push(casted_val);
        }

        // 3. Jika SELURUH data berhasil di-cast, perbarui data fisik di setiap Row
        for (row, new_val) in table.rows_mut().iter_mut().zip(new_values) {
            row.values_mut()[col_idx] = new_val; // Pastikan values_mut() atau helper setter ada di Row
        }

        // 4. Perbarui skema tabel
        table.schema_mut().modify_column_type(col_id, new_type)?;

        Ok(())
    }
}
