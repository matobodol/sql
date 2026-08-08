use std::sync::Arc;

use crate::{DomainError, Row, RowId, Schema, SqlValue, eval_expr};

pub(crate) fn validate_row(schema: &Schema, values: &[SqlValue]) -> Result<(), DomainError> {
    if values.len() != schema.columns().len() {
        return Err(DomainError::eval_error(format!(
            "Jumlah kolom tidak sesuai: mengharapkan {}, ditemukan {}",
            schema.columns().len(),
            values.len()
        )));
    }

    for (col, val) in schema.columns().iter().zip(values.iter()) {
        if val.is_null() {
            if !col.is_nullable() {
                return Err(DomainError::eval_error(format!(
                    "Kolom '{}' tidak boleh NULL",
                    col.name
                )));
            }
        } else if !val.matches_type(&col.sql_type) {
            return Err(DomainError::TypeMismatch {
                expected: Arc::from(format!("{:?}", col.sql_type).as_str()),
                found: Arc::from(format!("{:?}", val).as_str()),
            });
        }
    }

    // Fast-path: Skip evaluasi jika tidak ada CHECK constraint
    if !schema.has_check_constraints() {
        return Ok(());
    }

    let temp_row = Row::with_id(RowId::from(0u64), values.to_vec());

    for (col_idx, bound_expr) in schema.bound_column_checks() {
        let res = eval_expr(bound_expr, &temp_row)?;
        if !res.is_null() && res.as_ref() == &SqlValue::Bool(false) {
            return Err(DomainError::eval_error(format!(
                "Pelanggaran CHECK constraint pada kolom '{}'",
                schema.columns()[*col_idx].name
            )));
        }
    }

    for bound_expr in schema.bound_table_checks() {
        let res = eval_expr(bound_expr, &temp_row)?;
        if !res.is_null() && res.as_ref() == &SqlValue::Bool(false) {
            return Err(DomainError::eval_error(
                "Pelanggaran CHECK constraint pada tabel",
            ));
        }
    }

    Ok(())
}
