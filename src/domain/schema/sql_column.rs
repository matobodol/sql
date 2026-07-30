use crate::SqlType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnDef {
    pub name: String,
    pub sql_type: SqlType,
    pub nullable: bool,
}

impl ColumnDef {
    pub fn new(name: impl Into<String>, sql_type: SqlType, nullable: bool) -> Self {
        Self {
            name: name.into(),
            sql_type,
            nullable,
        }
    }
}
