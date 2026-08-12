use crate::DomainError;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::Path;

pub struct DiskStorage;

impl DiskStorage {
    /// Menyimpan data apa pun yang mendukung Serialize ke file biner di disk.
    /// Mengonversi error I/O atau bincode secara otomatis ke `DomainError::Storage`.
    pub fn save_to_file<T: Serialize>(path: &Path, data: &T) -> Result<(), DomainError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| DomainError::storage(e.to_string()))?;
        }

        let file = File::create(path).map_err(|e| DomainError::storage(e.to_string()))?;

        let writer = BufWriter::new(file);

        bincode::serialize_into(writer, data).map_err(|e| DomainError::storage(e.to_string()))
    }

    /// Membaca dan melakukan deserialisasi data dari file biner di disk.
    pub fn load_from_file<T>(path: &Path) -> Result<T, DomainError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let file = File::open(path).map_err(|e| DomainError::storage(e.to_string()))?;

        let reader = BufReader::new(file);

        bincode::deserialize_from(reader).map_err(|e| DomainError::storage(e.to_string()))
    }
}
