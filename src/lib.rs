pub mod api;
pub mod catalog;
pub mod disk;
pub mod error;
pub mod execution;
pub mod expression;
pub mod index;
pub mod logic;
pub mod schema;
pub mod storage;
pub mod types;
pub mod validator;

// Re-exports untuk publik
pub use catalog::{
    Database, DatabaseManager,
    id::{ColumnId, RowId, TableId},
};
pub use error::DomainError;
pub use execution::{AggregateFunc, OrderByExpr, SelectStmt, SortOrder};
pub use expression::{BinaryOp, Expr};
pub use schema::{Column, ColumnConstraint, ColumnPosition, Increment, Schema, TableConstraint};
pub use storage::{Row, TableContext};
pub use types::{Bool3VL, DataType, ValueType};

pub use api::DBM;
