use std::{collections::HashSet, sync::Arc};

use crate::{DdlAction, DomainError, TableAction, catalog::CatalogStore};

pub(crate) fn validate_alter_table(
    catalog: &CatalogStore,
    name: &str,
    actions: &[DdlAction],
) -> Result<(), DomainError> {
    // 1. Pastikan tabel tujuan ada terlebih dahulu
    let table_id = catalog.get_table_id(&name)?;

    // 2. Ambil skema awal untuk simulasi validasi batch
    let schema = catalog.get_schema(table_id)?;

    // Buat pelacak lokal untuk memvalidasi aksi multi-kolom dalam satu batch
    let mut active_columns: std::collections::HashSet<String> = schema
        .columns()
        .iter()
        .map(|c| c.name.to_string())
        .collect();

    for action in actions.iter() {
        match action {
            DdlAction::AddColumns { columns } => {
                for (col_name, _, _, _) in columns {
                    if active_columns.contains(col_name) {
                        return Err(DomainError::ColumnAlreadyExists(Arc::from(
                            col_name.as_str(),
                        )));
                    }
                    active_columns.insert(col_name.to_string());
                }
            }
            DdlAction::DropColumn { col_name } => {
                if !active_columns.contains(col_name) {
                    return Err(DomainError::eval_error(format!(
                        "Kolom '{col_name}' tidak ditemukan"
                    )));
                }
                active_columns.remove(col_name);
            }
            DdlAction::RenameColumn {
                old_col_name: old_name,
                new_col_name: new_name,
            } => {
                if !active_columns.contains(old_name) {
                    return Err(DomainError::eval_error(format!(
                        "Kolom '{old_name}' tidak ditemukan"
                    )));
                }
                if active_columns.contains(new_name) {
                    return Err(DomainError::ColumnAlreadyExists(Arc::from(
                        new_name.as_str(),
                    )));
                }
                active_columns.remove(old_name);
                active_columns.insert(new_name.to_string());
            }
            // Menangani varian SetDefault sesuai definisi struct
            DdlAction::ModifyColumnType { col_name, .. }
            | DdlAction::AddConstraint { col_name, .. }
            | DdlAction::DropConstraint { col_name, .. }
            | DdlAction::SetDefault { col_name, .. } => {
                if !active_columns.contains(col_name) {
                    return Err(DomainError::eval_error(format!(
                        "Kolom '{col_name}' tidak ditemukan"
                    )));
                }
            }
        }
    }

    Ok(())
}

pub(crate) fn validate_table_action(
    catalog: &CatalogStore,
    actions: &[TableAction],
) -> Result<(), DomainError> {
    // cek duplikat candidate
    let mut seen_col: HashSet<String> = HashSet::new();
    let mut seen_tbl: HashSet<String> = catalog
        .list_tables()
        .iter()
        .map(|tbl| tbl.clone())
        .collect();

    for action in actions.iter() {
        match action {
            TableAction::CreateTable {
                table_name,
                columns,
            } => {
                if seen_tbl.get(table_name).is_some() {
                    return Err(DomainError::TableAlreadyExists(Arc::from(
                        table_name.as_str(),
                    )));
                }
                for col in columns.iter() {
                    if !seen_col.insert(col.0.clone()) {
                        return Err(DomainError::exec_error(Arc::from(format!(
                            "ErrorCteateTable: duplicat candidat column name '{}'",
                            col.0.as_str()
                        ))));
                    }
                }
            }
            TableAction::DropTable { table_name } => {
                if !seen_tbl.remove(table_name) {
                    return Err(DomainError::TableNotFound(Arc::from(table_name.as_str())));
                }
            }
            TableAction::RenameTable {
                old_table_name,
                new_table_name,
            } => {
                if seen_tbl.get(old_table_name).is_none() {
                    return Err(DomainError::TableNotFound(Arc::from(
                        old_table_name.as_str(),
                    )));
                }
                if seen_tbl.get(new_table_name).is_some() {
                    return Err(DomainError::TableAlreadyExists(Arc::from(
                        new_table_name.as_str(),
                    )));
                }
            }
        }
    }

    Ok(())
}
