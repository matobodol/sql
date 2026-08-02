use crate::catalog::db_function::dml_action::{DmlAction, DmlResult, execute_dml};
use crate::domain::id::{ColumnId, IdGenerator, RowId, TableId};
use crate::domain::{ColumnConstraint, DomainError, Row, Schema};
use crate::index::IndexRegistry;
use crate::{AutoIncrement, SqlValue};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Table {
    id: TableId,
    name: String,
    schema: Schema,
    rows: Vec<Row>,
    /// Generator sekuensial internal untuk memproduksi RowId
    row_id_gen: IdGenerator,
    /// Registry indeks untuk BTreeIndex pada kolom-kolom ber-indeks
    index_registry: IndexRegistry,
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

        let mut table = Self {
            id,
            name: name.into(),
            schema,
            rows: Vec::new(),
            row_id_gen: IdGenerator::new(1),
            index_registry: IndexRegistry::new(),
            auto_increment_counters,
        };

        // Otomatis bangun BTreeIndex untuk kolom PRIMARY KEY dan UNIQUE
        table.build_indexes_from_schema();
        table
    }

    /// Memeriksa skema dan membuat BTreeIndex secara otomatis untuk UNIQUE/PRIMARY KEY
    fn build_indexes_from_schema(&mut self) {
        for col in self.schema.columns() {
            let is_unique = col.is_primary_key()
                || col
                    .constraints
                    .iter()
                    .any(|c| matches!(c, ColumnConstraint::Unique));

            if is_unique {
                let _ = self.index_registry.create_btree_index(col.id, true);
            }
        }
    }

    // --- GETTERS & SETTERS ---

    pub fn id(&self) -> TableId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
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

    pub fn rows_mut(&mut self) -> &mut Vec<Row> {
        &mut self.rows
    }

    pub fn index_registry(&self) -> &IndexRegistry {
        &self.index_registry
    }

    pub fn index_registry_mut(&mut self) -> &mut IndexRegistry {
        &mut self.index_registry
    }

    // --- HELPER INTERNAL UNTUK DML ENGINE ---

    /// Mengalokasikan dan mengembalikan RowId berikutnya secara thread-safe
    pub(crate) fn next_row_id(&self) -> RowId {
        self.row_id_gen.next_row_id()
    }

    pub(crate) fn auto_increment_counters(&self) -> &HashMap<ColumnId, i64> {
        &self.auto_increment_counters
    }

    pub(crate) fn auto_increment_counters_mut(&mut self) -> &mut HashMap<ColumnId, i64> {
        &mut self.auto_increment_counters
    }

    // --- API DML EKSTERNAL ---

    /// Pintu masuk utama untuk seluruh aksi DML (INSERT, UPDATE, DELETE)
    pub fn execute_dml(&mut self, action: DmlAction) -> Result<DmlResult, DomainError> {
        execute_dml(self, action)
    }

    /// Helper instan untuk Single Insert (Convenience Wrapper)
    pub fn insert(&mut self, row_values: Vec<SqlValue>) -> Result<usize, DomainError> {
        match self.execute_dml(DmlAction::Insert {
            rows: vec![row_values],
        })? {
            DmlResult::Inserted(count) => Ok(count),
            _ => unreachable!(),
        }
    }

    /// Helper instan untuk Bulk Insert (Convenience Wrapper)
    pub fn insert_batch(&mut self, rows: Vec<Vec<SqlValue>>) -> Result<usize, DomainError> {
        match self.execute_dml(DmlAction::Insert { rows })? {
            DmlResult::Inserted(count) => Ok(count),
            _ => unreachable!(),
        }
    }
}
