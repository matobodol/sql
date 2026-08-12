use serde::{Deserialize, Serialize};
use std::ops::Deref;

use crate::{RowId, ValueType};

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

    #[inline]
    pub fn values(&self) -> &[ValueType] {
        &self.values
    }

    #[inline]
    pub fn get_by_index(&self, index: usize) -> Option<&ValueType> {
        self.values.get(index)
    }
}

impl Deref for Row {
    type Target = [ValueType];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

//     ----   DEPRECATED   ----
// impl From<(RowId, Vec<ValueType>)> for Row {
//     fn from((id, values): (RowId, Vec<ValueType>)) -> Self {
//         Self::with_id(id, values)
//     }
// }
//
// impl Index<usize> for Row {
//     type Output = ValueType;
//
//     #[inline]
//     fn index(&self, index: usize) -> &Self::Output {
//         &self.values[index]
//     }
// }
//
//
// impl Row {
// fn set_id(&mut self, id: RowId) {
//     self.id = id;
// }
//
// fn into_values(self) -> Vec<ValueType> {
//     self.values
// }
//
// fn into_parts(self) -> (RowId, Vec<ValueType>) {
//     (self.id, self.values)
// }
//
// fn from_bytes(bytes: &[u8]) -> Result<Self, DomainError> {
//     rmp_serde::from_slice(bytes)
//         .map_err(|e| DomainError::eval_error(format!("Gagal mendeserialisasi row: {e}")))
// }
//
// fn to_bytes(&self) -> Result<Vec<u8>, DomainError> {
//     rmp_serde::to_vec(self)
//         .map_err(|e| DomainError::eval_error(format!("Gagal menserialisasi row: {e}")))
// }
// }
