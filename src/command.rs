use std::collections::HashMap;

use crate::{
    ColumnConstraint, ColumnId, Database, DomainError, Expr, Row, Schema, SelectStmt, SqlType,
    SqlValue,
    ddl_action::{
        create_table, drop_table, execute_add_columns, execute_add_constraint, execute_drop_column,
        execute_drop_constraint, execute_modify_column_type, execute_rename_column,
        execute_rename_table, execute_set_default,
    },
    dml_action::{handle_delete, handle_insert, handle_update},
    dql_action::{execute_select, show_tables},
};

#[derive(Debug, Clone, PartialEq)]
pub enum QueryResult {
    // -- Hasil eksekusi operasi DML --
    /// Jumlah baris yang berhasil disisipkan.
    Inserted(usize),
    /// Jumlah baris yang berhasil diperbarui.
    Updated(usize),
    /// Jumlah baris yang berhasil dihapus.
    Deleted(usize),

    // -- Hasil eksekusi operasi DQL --
    /// Hasil dari eksekusi Query DQL (`SELECT` / `SHOW`).
    Dql {
        /// Skema kolom dari tabel hasil query.
        schema: Schema,
        /// Daftar baris data yang dihasilkan.
        rows: Vec<Row>,
    },
    Ok,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ColumnPosition {
    Default,
    First,
    After(String),
}

/// Representasi aksi database tingkat tinggi (DDL, DML, DQL).
#[derive(Debug, Clone, PartialEq)]
pub enum CommandAction {
    ShowTables,
    CreateTable {
        name: String,
        columns: Vec<(String, SqlType, Vec<ColumnConstraint>)>,
    },
    DropTable {
        name: String,
    },
    AddColumn {
        name: String,
        columns: Vec<(String, SqlType, Vec<ColumnConstraint>, ColumnPosition)>,
    },
    DropColumn {
        name: String,
    },
    RenameColumn {
        old_name: String,
        new_name: String,
    },
    RenameTable {
        new_name: String,
    },
    ModifyColumnType {
        name: String,
        new_type: SqlType,
    },
    AddConstraint {
        col_name: String,
        constraint: ColumnConstraint,
    },
    DropConstraint {
        col_name: String,
        constraint: ColumnConstraint,
    },
    SetDefault {
        col_name: String,
        default_val: Option<SqlValue>,
    },

    // -- DML ACTION --
    /// BULK INSERT: Menyisipkan satu atau beberapa baris data ke tabel.
    Insert {
        rows: Vec<Vec<SqlValue>>,
    },

    /// UPDATE: Memperbarui nilai kolom berdasarkan kondisi predicate.
    Update {
        assignments: HashMap<ColumnId, Expr>,
        predicate: Option<Expr>,
    },

    /// DELETE: Menghapus baris berdasarkan kondisi predicate.
    Delete {
        predicate: Option<Expr>,
    },

    // -- DQL ACTION --
    Select {
        statements: SelectStmt,
    },
}

/// Eksekusi perintah database secara terpusat.
pub fn execute_command(
    db: &mut Database,
    table_name: &str,
    action: CommandAction,
) -> Result<QueryResult, DomainError> {
    match action {
        // -- DDL ACTION --
        CommandAction::CreateTable { name, columns } => {
            let (catalog, tables) = db.catalog_and_tables_mut();
            create_table(catalog, tables, &name, columns)?;
            Ok(QueryResult::Ok)
        }
        CommandAction::DropTable { name } => {
            let (catalog, tables) = db.catalog_and_tables_mut();
            drop_table(catalog, tables, &name)?;
            Ok(QueryResult::Ok)
        }
        CommandAction::AddColumn { name: _, columns } => {
            let (catalog, tables) = db.catalog_and_tables_mut();
            execute_add_columns(catalog, tables, table_name, columns)?;
            Ok(QueryResult::Ok)
        }
        CommandAction::DropColumn { name } => {
            let (catalog, tables) = db.catalog_and_tables_mut();
            execute_drop_column(catalog, tables, table_name, &name)?;
            Ok(QueryResult::Ok)
        }
        CommandAction::RenameColumn { old_name, new_name } => {
            execute_rename_column(db.catalog_mut(), table_name, &old_name, &new_name)?;
            Ok(QueryResult::Ok)
        }
        CommandAction::RenameTable { new_name } => {
            let (catalog, tables) = db.catalog_and_tables_mut();
            execute_rename_table(catalog, tables, table_name, &new_name)?;
            Ok(QueryResult::Ok)
        }
        CommandAction::ModifyColumnType { name, new_type } => {
            execute_modify_column_type(db.catalog_mut(), table_name, &name, new_type)?;
            Ok(QueryResult::Ok)
        }
        CommandAction::AddConstraint {
            col_name,
            constraint,
        } => {
            let (catalog, tables) = db.catalog_and_tables_mut();
            execute_add_constraint(catalog, tables, table_name, &col_name, constraint)?;
            Ok(QueryResult::Ok)
        }
        CommandAction::DropConstraint {
            col_name,
            constraint,
        } => {
            execute_drop_constraint(db.catalog_mut(), table_name, &col_name, &constraint)?;
            Ok(QueryResult::Ok)
        }
        CommandAction::SetDefault {
            col_name,
            default_val,
        } => {
            execute_set_default(db.catalog_mut(), table_name, &col_name, default_val)?;
            Ok(QueryResult::Ok)
        }

        // -- DML ACTION --
        CommandAction::Insert { rows } => {
            // Memindahkan kepemilikan `rows` tanpa clone
            let inserted_count = handle_insert(db, table_name, rows)?;
            Ok(QueryResult::Inserted(inserted_count))
        }
        CommandAction::Update {
            assignments,
            predicate,
        } => {
            let updated_count = handle_update(db, table_name, &assignments, predicate.as_ref())?;
            Ok(QueryResult::Updated(updated_count))
        }
        CommandAction::Delete { predicate } => {
            let deleted_count = handle_delete(db, table_name, predicate.as_ref())?;
            Ok(QueryResult::Deleted(deleted_count))
        }

        // -- DQL ACTION --
        CommandAction::Select { statements } => execute_select(db, table_name, statements),
        CommandAction::ShowTables => show_tables(db),
    }
}
