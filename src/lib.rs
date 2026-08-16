pub mod api;
pub mod api_command;
pub mod catalog;
pub mod disk;
pub mod error;
pub mod execution;
pub mod expression;
pub mod index;
pub mod logic;
pub mod schema;
pub mod table_store;
pub mod types;
pub mod validator;

// Re-exports untuk publik
pub use catalog::{
    Database, DatabaseManager,
    id::{ColumnId, RowId, TableId},
};
pub use error::DomainError;
pub use execution::{AggregateFunc, OrderByExpr, SortOrder, Statement};
pub use expression::{BinaryOp, Expr};
pub use schema::{Column, ColumnConstraint, ColumnPosition, Increment, Schema, TableConstraint};
pub use table_store::{Row, TableContext};
pub use types::{Bool3VL, DataType, ValueType};

pub use api::DBM;
