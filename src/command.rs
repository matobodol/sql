use std::collections::HashMap;

use crate::{
    ColumnConstraint, ColumnId, DataType, Database, DomainError, Expr, Row, Schema, SelectStmt,
    TableId, TableStorage, ValueType,
    catalog::CatalogStore,
    ddl_action::{
        apply_add_columns, apply_add_constraint, apply_create_table, apply_drop_column,
        apply_drop_constraint, apply_drop_table, apply_modify_column_type, apply_rename_column,
        apply_rename_table, apply_set_default,
    },
    dml_action::{handle_delete, handle_insert, handle_update},
    dql_action::{execute_select, execute_show_tables},
    validator::{validate_alter_table, validate_table_action},
};

#[derive(Debug, Clone, PartialEq)]
pub enum QueryResult {
    Inserted(usize),
    Updated(usize),
    Deleted(usize),
    Dql { schema: Schema, rows: Vec<Row> },
    OK,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ColumnPosition {
    Default,
    First,
    After(String),
}

/// Konsolidasi aksi tingkat tabel agar mendukung batch/multi-aksi secara seragam.
#[derive(Debug, Clone, PartialEq)]
pub enum TableAction {
    CreateTable {
        name: String,
        columns: Vec<(String, DataType, Vec<ColumnConstraint>)>,
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
pub enum DdlAction {
    AddColumns(Vec<(String, DataType, Vec<ColumnConstraint>, ColumnPosition)>),
    DropColumn(String),
    RenameColumn {
        old_name: String,
        new_name: String,
    },
    ModifyColumnType {
        col_name: String,
        new_type: DataType,
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
        default_val: Option<ValueType>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum DmlAction {
    Insert {
        rows: Vec<Vec<ValueType>>,
    },
    Update {
        assignments: HashMap<ColumnId, Expr>,
        predicate: Option<Expr>,
    },
    Delete {
        predicate: Option<Expr>,
    },
}

/// Representasi aksi database tingkat tinggi (DDL, DML, DQL) yang bersih dari duplikasi.
#[derive(Debug, Clone, PartialEq)]
pub enum CommandAction {
    ShowTables,
    /// Operasi tingkat tabel berbasis batch terpadu.
    TableAction {
        actions: Vec<TableAction>,
    },
    /// Konsolidasi seluruh operasi perubahan skema tabel ala standar SQL.
    AlterTable {
        name: String,
        actions: Vec<DdlAction>,
    },

    // -- DML ACTION --
    DmlAction {
        action: DmlAction,
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
    //
    let (catalog, tables) = db.catalog_and_tables_mut();

    match action {
        // -- DDL ACTION --
        CommandAction::TableAction { actions } => execute_table_action(catalog, tables, actions),
        CommandAction::AlterTable { name, actions } => {
            execute_alter_table(catalog, tables, &name, actions)
        }

        // -- DML ACTION --
        CommandAction::DmlAction { action } => execute_dml_action(db, table_name, action),

        // -- DQL ACTION --
        CommandAction::Select { statements } => execute_select(db, table_name, statements),
        CommandAction::ShowTables => execute_show_tables(catalog),
    }
}

fn execute_table_action(
    catalog: &mut CatalogStore,
    tables: &mut HashMap<TableId, TableStorage>,
    actions: Vec<TableAction>,
) -> Result<QueryResult, DomainError> {
    // === FASE 1: PRE-CHECK (Dry-Run Validasi) ===
    validate_table_action(&catalog, &actions)?;

    // === FASE 2: EKSEKUSI NYATA (Mutation) ===
    for action in actions {
        match action {
            TableAction::CreateTable { name, columns } => {
                apply_create_table(catalog, tables, &name, columns)?;
            }
            TableAction::DropTable { name } => {
                apply_drop_table(catalog, tables, &name)?;
            }
            TableAction::RenameTable { old_name, new_name } => {
                apply_rename_table(catalog, tables, &old_name, &new_name)?;
            }
        }
    }
    Ok(QueryResult::OK)
}

fn execute_alter_table(
    catalog: &mut CatalogStore,
    tables: &mut HashMap<TableId, TableStorage>,
    name: &str,
    actions: Vec<DdlAction>,
) -> Result<QueryResult, DomainError> {
    // === FASE 1: PRE-CHECK (Dry-Run Validasi Skema) ===
    validate_alter_table(catalog, &name, &actions)?;

    // === FASE 2: EKSEKUSI NYATA (Mutation) ===
    for action in actions {
        match action {
            DdlAction::AddColumns(columns) => {
                apply_add_columns(catalog, tables, &name, columns)?;
            }
            DdlAction::DropColumn(col_name) => {
                apply_drop_column(catalog, tables, &name, &col_name)?;
            }
            DdlAction::RenameColumn { old_name, new_name } => {
                apply_rename_column(catalog, tables, &name, &old_name, &new_name)?;
            }
            DdlAction::ModifyColumnType { col_name, new_type } => {
                apply_modify_column_type(catalog, tables, &name, &col_name, new_type)?;
            }
            DdlAction::AddConstraint {
                col_name,
                constraint,
            } => {
                apply_add_constraint(catalog, tables, &name, &col_name, constraint)?;
            }
            DdlAction::DropConstraint {
                col_name,
                constraint,
            } => {
                apply_drop_constraint(catalog, tables, &name, &col_name, &constraint)?;
            }
            DdlAction::SetDefault {
                col_name,
                default_val,
            } => {
                apply_set_default(catalog, tables, &name, &col_name, default_val)?;
            }
        }
    }
    Ok(QueryResult::OK)
}

fn execute_dml_action(
    db: &mut Database,
    table_name: &str,
    action: DmlAction,
) -> Result<QueryResult, DomainError> {
    match action {
        DmlAction::Insert { rows } => {
            let inserted_count = handle_insert(db, table_name, rows)?;
            Ok(QueryResult::Inserted(inserted_count))
        }
        DmlAction::Update {
            assignments,
            predicate,
        } => {
            let updated_count = handle_update(db, table_name, &assignments, predicate.as_ref())?;
            Ok(QueryResult::Updated(updated_count))
        }
        DmlAction::Delete { predicate } => {
            let deleted_count = handle_delete(db, table_name, predicate.as_ref())?;
            Ok(QueryResult::Deleted(deleted_count))
        }
    }
}
