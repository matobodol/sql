use std::collections::HashMap;

use crate::{
    ColumnConstraint, ColumnId, DataType, DomainError, Expr, Row, Schema, SelectStmt, TableContext,
    TableId, ValueType,
    catalog::CatalogStore,
    disk::{BufferPoolManager, TableHeap},
    index::IndexRegistry,
    logic::{
        apply_add_columns, apply_add_constraint, apply_create_table, apply_drop_column,
        apply_drop_constraint, apply_drop_table, apply_modify_column_type, apply_rename_column,
        apply_rename_table, apply_set_default, handle_delete, handle_insert, handle_update,
    },
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
        table_name: String,
        columns: Vec<(String, DataType, Vec<ColumnConstraint>)>,
    },
    DropTable {
        table_name: String,
    },
    RenameTable {
        old_table_name: String,
        new_table_name: String,
    },
}

/// Sub-tindakan yang valid di dalam pernyataan ALTER TABLE SQL standar.
#[derive(Debug, Clone, PartialEq)]
pub enum DdlAction {
    AddColumns {
        columns: Vec<(String, DataType, Vec<ColumnConstraint>, ColumnPosition)>,
    },
    DropColumn {
        col_name: String,
    },
    RenameColumn {
        old_col_name: String,
        new_col_name: String,
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
    DescribeTable {
        table_name: String,
    },
    /// Operasi tingkat tabel berbasis batch terpadu.
    TableAction {
        actions: Vec<TableAction>,
    },
    /// Konsolidasi seluruh operasi perubahan skema tabel ala standar SQL.
    AlterTable {
        table_name: String,
        actions: Vec<DdlAction>,
    },

    // -- DML ACTION --
    DmlAction {
        table_name: String,
        action: DmlAction,
    },
    // -- DQL ACTION --
    Select {
        table_name: String,
        statements: SelectStmt,
    },
}

pub(crate) fn execute_table_action(
    catalog: &mut CatalogStore,
    tables: &mut HashMap<TableId, TableContext>,
    db_path: &str, // Parameter tambahan untuk path folder database
    actions: Vec<TableAction>,
) -> Result<(), DomainError> {
    // === FASE 1: PRE-CHECK (Dry-Run Validasi) ===
    validate_table_action(&catalog, &actions)?;

    // === FASE 2: EKSEKUSI NYATA (Mutation) ===
    for action in actions {
        match action {
            TableAction::CreateTable {
                table_name,
                columns,
            } => {
                apply_create_table(catalog, tables, db_path, &table_name, columns)?;
            }
            TableAction::DropTable { table_name } => {
                apply_drop_table(catalog, tables, db_path, &table_name)?;
            }
            TableAction::RenameTable {
                old_table_name,
                new_table_name,
            } => {
                apply_rename_table(catalog, db_path, &old_table_name, &new_table_name)?;
            }
        }
    }
    Ok(())
}

pub(crate) fn execute_alter_table(
    catalog: &mut CatalogStore,
    tables: &mut HashMap<TableId, TableContext>,
    name: &str,
    actions: Vec<DdlAction>,
) -> Result<(), DomainError> {
    // === FASE 1: PRE-CHECK (Dry-Run Validasi Skema) ===
    validate_alter_table(catalog, name, &actions)?;

    // === FASE 2: EKSEKUSI NYATA (Mutation) ===
    for action in actions {
        match action {
            DdlAction::AddColumns { columns } => {
                apply_add_columns(catalog, tables, name, columns)?;
            }
            DdlAction::DropColumn { col_name } => {
                apply_drop_column(catalog, tables, name, &col_name)?;
            }
            DdlAction::RenameColumn {
                old_col_name,
                new_col_name,
            } => {
                apply_rename_column(catalog, name, &old_col_name, &new_col_name)?;
            }
            DdlAction::ModifyColumnType { col_name, new_type } => {
                apply_modify_column_type(catalog, name, &col_name, new_type)?;
            }
            DdlAction::AddConstraint {
                col_name,
                constraint,
            } => {
                apply_add_constraint(catalog, tables, name, &col_name, constraint)?;
            }
            DdlAction::DropConstraint {
                col_name,
                constraint,
            } => {
                apply_drop_constraint(catalog, name, &col_name, &constraint)?;
            }
            DdlAction::SetDefault {
                col_name,
                default_val,
            } => {
                apply_set_default(catalog, name, &col_name, default_val)?;
            }
        }
    }
    Ok(())
}

pub(crate) fn execute_dml_action(
    catalog: &CatalogStore,
    table_heap: &mut TableHeap,
    bpm: &mut BufferPoolManager,
    index_registry: &mut IndexRegistry,
    auto_increment_counters: &mut HashMap<ColumnId, i64>, // <-- Tangkap di sini
    table_id: TableId,
    action: DmlAction,
) -> Result<QueryResult, DomainError> {
    match action {
        DmlAction::Insert { rows } => {
            let inserted_count = handle_insert(
                catalog,
                table_heap,
                bpm,
                index_registry,
                auto_increment_counters, // <-- Teruskan ke handle_insert
                table_id,
                rows,
            )?;
            Ok(QueryResult::Inserted(inserted_count))
        }
        DmlAction::Update {
            assignments,
            predicate,
        } => {
            let updated_count = handle_update(
                catalog,
                table_heap,
                bpm,
                index_registry, // Teruskan ke handler
                table_id,
                &assignments,
                predicate.as_ref(),
            )?;
            Ok(QueryResult::Updated(updated_count))
        }
        DmlAction::Delete { predicate } => {
            let deleted_count = handle_delete(
                catalog,
                table_heap,
                bpm,
                index_registry, // Teruskan ke handler
                table_id,
                predicate.as_ref(),
            )?;
            Ok(QueryResult::Deleted(deleted_count))
        }
    }
}
