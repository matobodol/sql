use std::collections::BTreeMap;
use std::ops::Bound;

use crate::{DomainError, RowId, ValueType};

use super::traits::Index;

/// Implementasi BTree Index yang dioptimalkan memori dan nol alokasi sementara pada operasi pencarian/penghapusan.
#[derive(Debug, Clone)]
pub struct BTreeIndex {
    /// Pemetaan dari `SqlValue` ke kumpulan `RowId`
    map: BTreeMap<ValueType, Vec<RowId>>,
    /// Status apakah indeks mewajibkan nilai unik.
    is_unique: bool,
}

impl BTreeIndex {
    /// Inisialisasi BTree index baru.
    pub fn new(is_unique: bool) -> Self {
        Self {
            map: BTreeMap::new(),
            is_unique,
        }
    }
}

impl Index for BTreeIndex {
    fn clone_box(&self) -> Box<dyn Index> {
        Box::new(self.clone())
    }

    /// Memasukkan entri dengan lazy-cloning (kloning key hanya dilakukan jika key belum ada di BTree).
    fn insert(&mut self, key: &ValueType, row_id: RowId) -> Result<(), DomainError> {
        // Cek terlebih dahulu apakah key sudah ada untuk menghindari kloning key di awal
        if let Some(rows) = self.map.get_mut(key) {
            // Sesuai standar SQL: Hanya nilai NON-NULL yang diperiksa keunikannya
            if self.is_unique && !key.is_null() {
                return Err(DomainError::invalid_expr(format!(
                    "Pelanggaran keunikan indeks BTree pada nilai '{:?}'",
                    key
                )));
            }

            if !rows.contains(&row_id) {
                rows.push(row_id);
            }
            return Ok(());
        }

        // Kunci belum ada, lakukan alokasi kloning key sekali saja
        self.map.insert(key.clone(), vec![row_id]);
        Ok(())
    }

    /// Menghapus `RowId` tanpa melakukan kloning `SqlValue` (Zero-Allocation Remove).
    fn remove(&mut self, key: &ValueType, row_id: RowId) -> Result<(), DomainError> {
        // Gunakan get_mut berbasis referensi &SqlValue tanpa entry API yang butuh owned key
        if let Some(rows) = self.map.get_mut(key) {
            rows.retain(|&id| id != row_id);
            if rows.is_empty() {
                self.map.remove(key);
            }
        }
        Ok(())
    }

    /// Mengembalikan borrowed slice `&[RowId]` langsung dari BTreeMap tanpa alokasi `Vec` baru.
    fn lookup(&self, key: &ValueType) -> &[RowId] {
        self.map.get(key).map(|vec| vec.as_slice()).unwrap_or(&[])
    }

    fn range_lookup(&self, min: Bound<&ValueType>, max: Bound<&ValueType>) -> Vec<RowId> {
        self.map
            .range((min, max))
            .flat_map(|(_, rows)| rows.iter().copied())
            .collect()
    }

    #[inline]
    fn is_unique(&self) -> bool {
        self.is_unique
    }

    #[inline]
    fn clear(&mut self) {
        self.map.clear();
    }
}
