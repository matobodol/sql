use std::collections::HashMap;

use crate::{
    ColumnConstraint, ColumnId, Database, DomainError, Expr, Row, Schema, SelectStmt, SqlType,
    SqlValue,
    ddl_action::{
        create_table, create_table_action, drop_table, execute_add_columns, execute_add_constraint,
        execute_drop_column, execute_drop_constraint, execute_modify_column_type,
        execute_rename_column, execute_rename_table, execute_set_default,
    },
    dml_action::{handle_delete, handle_insert, handle_update},
    dql_action::execute_select,
    show::show_tables,
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

#[derive(Debug, Clone, PartialEq)]
pub enum TableAction {
    CreateTable {
        name: String,
        columns: Vec<(String, SqlType, Vec<ColumnConstraint>)>,
    },
    DropTable {
        name: String,
    },
    RenameTable {
        old_name: String,
        new_name: String,
    },
}
/// Sub-tindakan yang valid di dalam pernyataan ALTER TABLE SQL standar.
#[derive(Debug, Clone, PartialEq)]
pub enum AlterTableAction {
    AddColumns(Vec<(String, SqlType, Vec<ColumnConstraint>, ColumnPosition)>),
    DropColumn(String),
    RenameColumn {
        old_name: String,
        new_name: String,
    },
    ModifyColumnType {
        col_name: String,
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
}

/// Representasi aksi database tingkat tinggi (DDL, DML, DQL).
#[derive(Debug, Clone, PartialEq)]
pub enum CommandAction {
    // -- table operation
    ShowTables,
    TableAction {
        actions: Vec<TableAction>,
    },
    CreateTable {
        name: String,
        columns: Vec<(String, SqlType, Vec<ColumnConstraint>)>,
    },
    DropTable {
        name: String,
    },
    RenameTable {
        old_name: String,
        new_name: String,
    },
    // ddl operation
    /// Konsolidasi seluruh operasi perubahan skema tabel ala standar SQL.
    AlterTable {
        name: String,
        actions: Vec<AlterTableAction>,
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
        // -- TABLE ACTION
        CommandAction::TableAction { actions } => {
            let (catalog, tables) = db.catalog_and_tables_mut();
            for action in actions {
                match action {
                    TableAction::CreateTable { name, columns } => {
                        create_table_action(catalog, tables, &name, columns)?;
                    }
                    TableAction::DropTable { name } => {
                        drop_table(catalog, tables, &name)?;
                    }
                    TableAction::RenameTable { old_name, new_name } => {
                        execute_rename_table(catalog, tables, &old_name, &new_name)?;
                    }
                }
            }
            Ok(QueryResult::Ok)
        }
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
        CommandAction::RenameTable { old_name, new_name } => {
            let (catalog, tables) = db.catalog_and_tables_mut();
            execute_rename_table(catalog, tables, &old_name, &new_name)?;
            Ok(QueryResult::Ok)
        }
        // -- DDL ACTION --
        CommandAction::AlterTable { name, actions } => {
            let (catalog, tables) = db.catalog_and_tables_mut();
            for action in actions {
                match action {
                    AlterTableAction::AddColumns(columns) => {
                        execute_add_columns(catalog, tables, &name, columns)?;
                    }
                    AlterTableAction::DropColumn(col_name) => {
                        execute_drop_column(catalog, tables, &name, &col_name)?;
                    }
                    AlterTableAction::RenameColumn { old_name, new_name } => {
                        execute_rename_column(catalog, tables, &name, &old_name, &new_name)?;
                    }
                    AlterTableAction::ModifyColumnType { col_name, new_type } => {
                        execute_modify_column_type(catalog, tables, &name, &col_name, new_type)?;
                    }
                    AlterTableAction::AddConstraint {
                        col_name,
                        constraint,
                    } => {
                        execute_add_constraint(catalog, tables, &name, &col_name, constraint)?;
                    }
                    AlterTableAction::DropConstraint {
                        col_name,
                        constraint,
                    } => {
                        execute_drop_constraint(catalog, tables, &name, &col_name, &constraint)?;
                    }
                    AlterTableAction::SetDefault {
                        col_name,
                        default_val,
                    } => {
                        execute_set_default(catalog, tables, &name, &col_name, default_val)?;
                    }
                }
            }
            Ok(QueryResult::Ok)
        }

        // -- DML ACTION --
        CommandAction::Insert { rows } => {
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
        CommandAction::ShowTables => {
            let (catalog, _) = db.catalog_and_tables_mut();
            show_tables(catalog)
        }
    }
}
