use std::collections::HashSet;

use crate::{DataType, DomainError};

/// Memvalidasi definisi SqlType, memastikan tidak ada varian Enum yang duplikat
pub fn validate_enum_variants(sqltype: &DataType) -> Result<(), DomainError> {
    if let DataType::Enum { name, variants } = sqltype {
        let mut seen = HashSet::with_capacity(variants.len());

        for variant in variants {
            if !seen.insert(variant) {
                return Err(DomainError::eval_error(format!(
                    "Definisi Enum '{name}' tidak valid: varian '{variant}' terdefinisi lebih dari sekali"
                )));
            }
        }
    }

    Ok(())
}
