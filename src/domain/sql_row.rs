use serde::{Deserialize, Serialize};
use std::ops::Index;

use super::domain_error::DomainError;
use super::id::RowId;
use super::schema::Schema;
use super::sql_value::SqlValue;

/// Representasi satu baris data (tuple) di dalam tabel database.
///
/// Menyimpan `RowId` imut secara permanen untuk menjamin konsistensi
/// pemetaan indeks B-Tree dan persistensi penyimpanan ke disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Row {
    /// Identitas unik permanen dari baris data.
    id: RowId,
    /// Kumpulan nilai data (SqlValue) untuk setiap kolom.
    values: Vec<SqlValue>,
}

impl Row {
    /// Membuat instance `Row` baru tanpa ID spesifik (default `RowId(0)`).
    ///
    /// *Catatan:* Sebaiknya gunakan `Row::with_id` saat menyisipkan baris ke dalam tabel.
    pub fn new(values: Vec<SqlValue>) -> Self {
        Self {
            id: RowId::from(0u64),
            values,
        }
    }

    /// Membuat instance `Row` baru dengan `RowId` eksplisit dan permanen.
    pub fn with_id(id: RowId, values: Vec<SqlValue>) -> Self {
        Self { id, values }
    }

    /// Mengambil `RowId` permanen dari baris ini.
    pub fn id(&self) -> RowId {
        self.id
    }

    /// Mengatur `RowId` baru untuk baris ini.
    pub fn set_id(&mut self, id: RowId) {
        self.id = id;
    }

    /// Mengambil referensi ke seluruh nilai kolom di dalam baris.
    pub fn values(&self) -> &[SqlValue] {
        &self.values
    }

    /// Mengambil referensi mutable ke seluruh nilai kolom di dalam baris.
    pub fn values_mut(&mut self) -> &mut Vec<SqlValue> {
        &mut self.values
    }

    /// Mengambil nilai kolom berdasarkan posisi indeks kolom.
    pub fn get_by_index(&self, index: usize) -> Option<&SqlValue> {
        self.values.get(index)
    }

    /// Mengambil nilai kolom berdasarkan nama kolom dengan bantuan `Schema`.
    pub fn get_by_name<'a>(
        &'a self,
        schema: &Schema,
        col_name: &str,
    ) -> Result<&'a SqlValue, DomainError> {
        let idx = schema.index_of(col_name).ok_or_else(|| {
            DomainError::ColumnNotFound(format!("Kolom '{col_name}' tidak ditemukan pada skema"))
        })?;

        self.get_by_index(idx).ok_or_else(|| {
            DomainError::EvaluationError(format!("Data pada indeks {idx} tidak ditemukan"))
        })
    }

    /// Mengonsumsi `Row` dan mengembalikan nilai kolom internalnya (`Vec<SqlValue>`).
    pub fn into_values(self) -> Vec<SqlValue> {
        self.values
    }

    /// Mengonsumsi `Row` dan mengembalikan tuple berisi `(RowId, Vec<SqlValue>)`.
    pub fn into_parts(self) -> (RowId, Vec<SqlValue>) {
        (self.id, self.values)
    }

    /// Menghapus dan mengembalikan nilai kolom pada indeks tertentu (digunakan saat `DROP COLUMN`).
    pub fn remove(&mut self, index: usize) -> Option<SqlValue> {
        if index < self.values.len() {
            Some(self.values.remove(index))
        } else {
            None
        }
    }

    /// Menambahkan nilai kolom baru ke posisi akhir baris (digunakan saat `ADD COLUMN`).
    pub fn push(&mut self, value: SqlValue) {
        self.values.push(value);
    }

    /// Mendeserialisasi slice byte mentah menjadi instance `Row`.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DomainError> {
        bincode::deserialize(bytes)
            .map_err(|e| DomainError::EvaluationError(format!("Gagal mendeserialisasi row: {e}")))
    }

    /// Menserialisasi instance `Row` menjadi representasi vektor byte (`Vec<u8>`).
    pub fn to_bytes(&self) -> Result<Vec<u8>, DomainError> {
        bincode::serialize(self)
            .map_err(|e| DomainError::EvaluationError(format!("Gagal menserialisasi row: {e}")))
    }
}

/// Konversi dari `Vec<SqlValue>` ke `Row` dengan default `RowId(0)`.
impl From<Vec<SqlValue>> for Row {
    fn from(values: Vec<SqlValue>) -> Self {
        Self::new(values)
    }
}

/// Konversi dari tuple `(RowId, Vec<SqlValue>)` ke `Row`.
impl From<(RowId, Vec<SqlValue>)> for Row {
    fn from((id, values): (RowId, Vec<SqlValue>)) -> Self {
        Self::with_id(id, values)
    }
}

/// Memungkinkan pembuatan `Row` langsung dari Iterator `SqlValue`.
impl FromIterator<SqlValue> for Row {
    fn from_iter<T: IntoIterator<Item = SqlValue>>(iter: T) -> Self {
        Self::new(iter.into_iter().collect())
    }
}

/// Pengaksesan nilai kolom menggunakan indeks array (misal: `let val = &row[0];`).
impl Index<usize> for Row {
    type Output = SqlValue;

    fn index(&self, index: usize) -> &Self::Output {
        &self.values[index]
    }
}
