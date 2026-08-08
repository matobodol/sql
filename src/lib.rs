pub mod catalog;
pub mod command; // Rename dari coredb_command
pub mod database;
pub mod error;
pub mod execution;
pub mod expression; // Rename dari expresi
pub mod index;
pub mod query_logic;
pub mod schema;
pub mod storage;
pub mod types;
pub mod validator;

// Re-exports untuk publik
pub use catalog::id::{ColumnId, RowId, TableId};
pub use command::{ColumnPosition, CommandAction, DdlAction, QueryResult, TableAction};
pub use database::Database;
pub use error::DomainError;
pub use execution::*;
pub use expression::*;
pub use index::*;
pub use query_logic::*;
pub use schema::*;
pub use storage::*;
pub use types::*;
