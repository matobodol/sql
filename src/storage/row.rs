use serde::{Deserialize, Serialize};
use std::ops::{Deref, Index};

use crate::{DomainError, RowId, ValueType};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Row {
    id: RowId,
    values: Vec<ValueType>,
}

impl Row {
    #[inline]
    pub fn with_id(id: RowId, values: Vec<ValueType>) -> Self {
        Self { id, values }
    }

    #[inline]
    pub fn id(&self) -> RowId {
        self.id
    }

    pub fn set_id(&mut self, id: RowId) {
        self.id = id;
    }

    #[inline]
    pub fn values(&self) -> &[ValueType] {
        &self.values
    }

    #[inline]
    pub fn get_by_index(&self, index: usize) -> Option<&ValueType> {
        self.values.get(index)
    }

    pub fn into_values(self) -> Vec<ValueType> {
        self.values
    }

    pub fn into_parts(self) -> (RowId, Vec<ValueType>) {
        (self.id, self.values)
    }

    /// Menyisipkan nilai ke indeks tertentu dengan validasi batas yang ketat.
    pub(crate) fn insert(&mut self, index: usize, value: ValueType) -> Result<(), DomainError> {
        if index > self.values.len() {
            return Err(DomainError::eval_error(format!(
                "Indeks kolom di luar batas: {index} (panjang row: {})",
                self.values.len()
            )));
        }
        self.values.insert(index, value);
        Ok(())
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DomainError> {
        rmp_serde::from_slice(bytes)
            .map_err(|e| DomainError::eval_error(format!("Gagal mendeserialisasi row: {e}")))
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, DomainError> {
        rmp_serde::to_vec(self)
            .map_err(|e| DomainError::eval_error(format!("Gagal menserialisasi row: {e}")))
    }
}

impl From<(RowId, Vec<ValueType>)> for Row {
    fn from((id, values): (RowId, Vec<ValueType>)) -> Self {
        Self::with_id(id, values)
    }
}

impl Index<usize> for Row {
    type Output = ValueType;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.values[index]
    }
}

impl Deref for Row {
    type Target = [ValueType];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.values
    }
}
