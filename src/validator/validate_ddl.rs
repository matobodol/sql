use std::sync::Arc;

use crate::{DdlAction, DomainError, TableAction, catalog::CatalogStore};

pub(crate) fn validate_alter_table(
    catalog: &CatalogStore,
    name: &str,
    actions: &[DdlAction],
) -> Result<(), DomainError> {
    // 1. Pastikan tabel tujuan ada terlebih dahulu
    let table_id = catalog
        .get_table_id(&name)
        .ok_or_else(|| DomainError::TableNotFound(Arc::from(name)))?;

    // 2. Ambil skema awal untuk simulasi validasi batch
    let schema = catalog.get_schema(table_id)?;

    // Buat pelacak lokal (case-insensitive) untuk memvalidasi aksi multi-kolom dalam satu batch
    let mut active_columns: std::collections::HashSet<String> = schema
        .columns()
        .iter()
        .map(|c| c.name.to_lowercase())
        .collect();

    for action in actions.iter() {
        match action {
            DdlAction::AddColumns(columns) => {
                for (col_name, _, _, _) in columns {
                    let col_lower = col_name.to_lowercase();
                    if active_columns.contains(&col_lower) {
                        return Err(DomainError::ColumnAlreadyExists(Arc::from(
                            col_name.as_str(),
                        )));
                    }
                    active_columns.insert(col_lower);
                }
            }
            DdlAction::DropColumn(col_name) => {
                let col_lower = col_name.to_lowercase();
                if !active_columns.contains(&col_lower) {
                    return Err(DomainError::eval_error(format!(
                        "Kolom '{col_name}' tidak ditemukan"
                    )));
                }
                active_columns.remove(&col_lower);
            }
            DdlAction::RenameColumn { old_name, new_name } => {
                let old_lower = old_name.to_lowercase();
                let new_lower = new_name.to_lowercase();
                if !active_columns.contains(&old_lower) {
                    return Err(DomainError::eval_error(format!(
                        "Kolom '{old_name}' tidak ditemukan"
                    )));
                }
                if active_columns.contains(&new_lower) {
                    return Err(DomainError::ColumnAlreadyExists(Arc::from(
                        new_name.as_str(),
                    )));
                }
                active_columns.remove(&old_lower);
                active_columns.insert(new_lower);
            }
            // Menangani varian SetDefault sesuai definisi struct
            DdlAction::ModifyColumnType { col_name, .. }
            | DdlAction::AddConstraint { col_name, .. }
            | DdlAction::DropConstraint { col_name, .. }
            | DdlAction::SetDefault { col_name, .. } => {
                let col_lower = col_name.to_lowercase();
                if !active_columns.contains(&col_lower) {
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
    for action in actions.iter() {
        match action {
            TableAction::CreateTable { name, .. } => {
                if catalog.get_table_id(name).is_some() {
                    return Err(DomainError::TableAlreadyExists(Arc::from(name.as_str())));
                }
            }
            TableAction::DropTable { name } => {
                if catalog.get_table_id(name).is_none() {
                    return Err(DomainError::TableNotFound(Arc::from(name.as_str())));
                }
            }
            TableAction::RenameTable { old_name, new_name } => {
                if catalog.get_table_id(old_name).is_none() {
                    return Err(DomainError::TableNotFound(Arc::from(old_name.as_str())));
                }
                if catalog.get_table_id(new_name).is_some() {
                    return Err(DomainError::TableAlreadyExists(Arc::from(
                        new_name.as_str(),
                    )));
                }
            }
        }
    }

    Ok(())
}
