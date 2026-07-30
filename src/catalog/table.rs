use crate::AutoIncrement;
use crate::domain::id::{ColumnId, TableId};
use crate::domain::{ColumnConstraint, DomainError, Row, Schema, SqlValue};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Table {
    id: TableId,
    name: String,
    schema: Schema,
    rows: Vec<Row>,
    /// Menggunakan ColumnId sebagai Key agar imun terhadap RENAME COLUMN!
    auto_increment_counters: HashMap<ColumnId, i64>,
}

impl Table {
    pub fn new(id: TableId, name: impl Into<String>, schema: Schema) -> Self {
        let mut auto_increment_counters = HashMap::new();

        // Inisialisasi counter berdasarkan `start` masing-masing kolom
        for col in schema.columns() {
            if let Some(AutoIncrement::Enabled { start, .. }) = col.auto_increment_config() {
                auto_increment_counters.insert(col.id, *start);
            }
        }

        Self {
            id,
            name: name.into(),
            schema,
            rows: Vec::new(),
            auto_increment_counters,
        }
    }

    pub fn id(&self) -> TableId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    pub fn schema_mut(&mut self) -> &mut Schema {
        &mut self.schema
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// Memasukkan data baru (INSERT) ke dalam tabel
    pub fn insert(&mut self, mut row_values: Vec<SqlValue>) -> Result<(), DomainError> {
        let columns = self.schema.columns();

        // 1. Pad array nilai jika nilainya kurang dari jumlah kolom di schema
        if row_values.len() < columns.len() {
            row_values.resize(columns.len(), SqlValue::Null);
        }

        // 2. Tangani AutoIncrement & Default Values
        for (i, col) in columns.iter().enumerate() {
            let is_null = row_values[i].is_null();

            // A. AutoIncrement Priority (Lookup via col.id)
            if col.is_auto_increment() && is_null {
                let counter = self
                    .auto_increment_counters
                    .get_mut(&col.id)
                    .expect("Counter auto-increment harusnya sudah diinisialisasi");

                // 1. Set nilai saat ini
                row_values[i] = SqlValue::Int(*counter);

                // 2. Ambil nilai step dari konfigurasi
                let step = match col.auto_increment_config() {
                    Some(AutoIncrement::Enabled { step, .. }) => *step,
                    _ => 1,
                };

                // 3. Tambahkan counter sesuai step
                *counter += step;
            }
            // B. Jika user mengisi nilai manual (tidak Null) pada kolom AutoIncrement:
            else if col.is_auto_increment() && !is_null {
                if let SqlValue::Int(manual_val) = row_values[i] {
                    if let Some(counter) = self.auto_increment_counters.get_mut(&col.id) {
                        if manual_val >= *counter {
                            let step = match col.auto_increment_config() {
                                Some(AutoIncrement::Enabled { step, .. }) => *step,
                                _ => 1,
                            };
                            // Sesuaikan counter agar lompat ke atas nilai manual user
                            *counter = manual_val + step;
                        }
                    }
                }
            }
            // C. Default Value Fallback
            else if is_null {
                if let Some(default_val) = col.default_value() {
                    row_values[i] = default_val.clone();
                }
            }
        }

        // 3. Validasi Keunikan Data (PRIMARY KEY & UNIQUE)
        for (i, col) in columns.iter().enumerate() {
            if col.is_primary_key()
                || col
                    .constraints
                    .iter()
                    .any(|c| matches!(c, ColumnConstraint::Unique))
            {
                let new_val = &row_values[i];

                if !new_val.is_null() {
                    let is_duplicate = self
                        .rows
                        .iter()
                        .any(|existing_row| existing_row.values().get(i) == Some(new_val));

                    if is_duplicate {
                        return Err(DomainError::EvaluationError(format!(
                            "Pelanggaran UNIQUE / PRIMARY KEY constraint pada kolom '{}' dengan nilai '{:?}'",
                            col.name, new_val
                        )));
                    }
                }
            }
        }

        // 4. Validasi Schema (Tipe data, NOT NULL, CHECK constraint)
        self.schema.validate_row(&row_values)?;

        // 5. Simpan baris data jika lolos semua validasi
        let row = Row::new(row_values);
        self.rows.push(row);
        Ok(())
    }
}
