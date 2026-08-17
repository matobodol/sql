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

impl From<Row> for Vec<ValueType> {
    #[inline]
    fn from(row: Row) -> Self {
        row.values
    }
}
