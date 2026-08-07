use serde::{Deserialize, Serialize};

use crate::{ColumnConstraint, SqlType, SqlValue, id::ColumnId, schema::AutoIncrement};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Column {
    pub id: ColumnId, // Source of Truth Identifier
    pub name: String, // Logical display name
    pub sql_type: SqlType,
    pub constraints: Vec<ColumnConstraint>,
}

impl Column {
    pub fn new(id: ColumnId, name: impl Into<String>, sql_type: SqlType) -> Self {
        Self {
            id,
            name: name.into(),
            sql_type,
            constraints: Vec::new(),
        }
    }

    pub fn with_constraints(
        id: ColumnId,
        name: impl Into<String>,
        sql_type: SqlType,
        constraints: Vec<ColumnConstraint>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            sql_type,
            constraints,
        }
    }

    /// Kolom TIDAK NULL jika memiliki constraint NotNull atau PrimaryKey.
    pub fn is_nullable(&self) -> bool {
        !self
            .constraints
            .iter()
            .any(|c| matches!(c, ColumnConstraint::NotNull | ColumnConstraint::PrimaryKey))
    }

    /// Memeriksa apakah kolom ini merupakan Primary Key
    pub fn is_primary_key(&self) -> bool {
        self.constraints
            .iter()
            .any(|c| matches!(c, ColumnConstraint::PrimaryKey))
    }

    /// Helper untuk memeriksa apakah kolom ini memiliki constraint AutoIncrement
    pub fn auto_increment_config(&self) -> Option<&AutoIncrement> {
        self.constraints.iter().find_map(|c| match c {
            ColumnConstraint::AutoIncrement(cfg @ AutoIncrement::Enabled { .. }) => Some(cfg),
            _ => None,
        })
    }

    /// Memeriksa apakah kolom menggunakan AutoIncrement
    pub fn is_auto_increment(&self) -> bool {
        self.auto_increment_config().is_some()
    }

    /// Mengambil nilai default jika ada
    pub fn default_value(&self) -> Option<&SqlValue> {
        self.constraints.iter().find_map(|c| match c {
            ColumnConstraint::Default(val) => Some(val),
            _ => None,
        })
    }
}
