use crate::{
    CommandAction, Database, DomainError, QueryResult, UserManager, storage::storage::DiskStorage,
};
use std::{collections::HashMap, path::Path};

type UserName = String;
type DatabaseName = String;

#[derive(Debug, Default)]
pub struct DatabaseManager {
    current_user: Option<String>,
    active_db: Option<String>,
    user_manager: UserManager,
    // Setiap database memiliki catalog dan tabelnya sendiri secara terisolasi
    databases: HashMap<(UserName, DatabaseName), Database>,
}

impl DatabaseManager {
    // ==========================================
    // KONSTRUKTOR & HELPER
    // ==========================================
    pub fn new() -> Self {
        // Pastikan direktori fisik untuk user 'root' selalu ada di disk
        let _ = std::fs::create_dir_all("Data_base/root");

        let mut manager = Self {
            current_user: Some("root".to_string()),
            active_db: None,
            user_manager: UserManager::new(),
            databases: HashMap::new(),
        };

        // Otomatis load data user dari disk jika file global_users.bin sudah ada
        let _ = manager.load_users();

        manager
    }

    #[inline]
    pub fn active_db_name(&self) -> Option<&str> {
        self.active_db.as_deref()
    }

    #[inline]
    fn current_username(&self) -> Result<&str, DomainError> {
        self.current_user
            .as_deref()
            .ok_or_else(|| DomainError::catalog("Tidak ada user yang sedang login."))
    }

    pub fn use_database(&mut self, db_name: &str) -> Result<(), DomainError> {
        let username = self.current_username()?.to_lowercase();
        let db_key = (username.clone(), db_name.to_lowercase());

        let is_admin = self.user_manager.is_admin(&username)?;

        // === LAZY LOADING ===
        // Jika database belum ada di dalam HashMap memori, coba muat dari disk
        if !self.databases.contains_key(&db_key) {
            // Memanggil fungsi load_from_disk yang membaca metadata.bin & file .db[span_4](start_span)[span_4](end_span)
            match Database::load_from_disk(&username, db_name) {
                Ok(db) => {
                    self.databases.insert(db_key.clone(), db);
                }
                Err(_) => {
                    return Err(DomainError::DatabaseNotFound(db_name.into()));
                }
            }
        }

        let target_key = if is_admin {
            self.databases
                .keys()
                .find(|(_, d)| d == &db_name.to_lowercase())
                .cloned()
                .ok_or_else(|| DomainError::DatabaseNotFound(db_name.into()))?
        } else {
            if !self.databases.contains_key(&db_key) {
                return Err(DomainError::DatabaseNotFound(db_name.into()));
            }
            db_key
        };

        self.active_db = Some(target_key.1);
        Ok(())
    }

    pub fn active_database_mut(&mut self) -> Result<&mut Database, DomainError> {
        let username = self.current_username()?.to_lowercase();
        let active_db_name = self
            .active_db
            .as_ref()
            .ok_or(DomainError::NoActiveDatabase)?;
        let db_key = active_db_name.to_lowercase();

        let is_admin = self.user_manager.is_admin(&username)?;

        let target_key = if is_admin {
            self.databases
                .keys()
                .find(|(_, d)| d == &db_key)
                .cloned()
                .ok_or_else(|| DomainError::DatabaseNotFound(active_db_name.clone().into()))?
        } else {
            (username, db_key)
        };

        self.databases
            .get_mut(&target_key)
            .ok_or_else(|| DomainError::DatabaseNotFound(active_db_name.clone().into()))
    }
}

impl DatabaseManager {
    // ==========================================
    // DISK MANAGEMENT
    // ==========================================
    const GLOBAL_USER_PATH: &str = "Data_base/global_users.bin";

    pub fn save_users(&self) -> Result<(), DomainError> {
        DiskStorage::save_to_file(Path::new(Self::GLOBAL_USER_PATH), &self.user_manager)
    }

    pub fn load_users(&mut self) -> Result<(), DomainError> {
        let path = Path::new(Self::GLOBAL_USER_PATH);
        if path.exists() {
            self.user_manager = DiskStorage::load_from_file(path)?;
        }
        Ok(())
    }
}

impl DatabaseManager {
    // ==========================================
    // DATABASE MANAGEMENT
    // ==========================================
    pub fn create_database(&mut self, db_name: &str) -> Result<(), DomainError> {
        let username = self.current_username()?;
        let composite_key = (username.to_lowercase(), db_name.to_lowercase());

        if self.databases.contains_key(&composite_key) {
            return Err(DomainError::DatabaseAlreadyExists(db_name.into()));
        }

        // Mengirim username dan db_name ke constructor Database
        let db = Database::new(username, db_name);

        self.databases.insert(composite_key, db);
        Ok(())
    }

    pub fn rename_database(
        &mut self,
        old_db_name: &str,
        new_db_name: &str,
    ) -> Result<(), DomainError> {
        let username = self.current_username()?.to_lowercase();
        let old_key = (username.clone(), old_db_name.to_lowercase());
        let new_key = (username.clone(), new_db_name.to_lowercase());

        // Pastikan nama baru belum dipakai
        if self.databases.contains_key(&new_key) {
            return Err(DomainError::DatabaseAlreadyExists(new_db_name.into()));
        }

        // Pastikan database lama ada (muat ke memori jika belum)
        if !self.databases.contains_key(&old_key) {
            self.use_database(old_db_name)?;
        }

        // 1. Pindahkan data di map memori (`HashMap`)
        if let Some(db) = self.databases.remove(&old_key) {
            self.databases.insert(new_key, db);
        } else {
            return Err(DomainError::DatabaseNotFound(old_db_name.into()));
        }

        // 2. Jika database yang direname sedang aktif, perbarui active_db
        if self
            .active_db
            .as_deref()
            .map_or(false, |active| active.eq_ignore_ascii_case(old_db_name))
        {
            self.active_db = Some(new_db_name.to_string());
        }

        // 3. Ubah nama folder fisik di disk secara serentak (`Data_base/{username}/{old_name}` -> `{new_name}`)
        let old_dir = format!("Data_base/{}/{}", username, old_db_name.to_lowercase());
        let new_dir = format!("Data_base/{}/{}", username, new_db_name.to_lowercase());

        let old_path = Path::new(&old_dir);
        let new_path = Path::new(&new_dir);

        if old_path.exists() {
            std::fs::rename(old_path, new_path).map_err(|e| {
                DomainError::storage(format!("Gagal merename folder database di disk: {e}"))
            })?;
        }

        Ok(())
    }

    pub fn drop_database(&mut self, db_name: &str) -> Result<(), DomainError> {
        let username = self.current_username()?.to_lowercase();
        let db_key = (username.clone(), db_name.to_lowercase());

        // 1. Pastikan database ada di memori atau disk
        if !self.databases.contains_key(&db_key) {
            // Coba lazy load dulu untuk memastikannya
            let _ = self.use_database(db_name);
        }

        // Hapus dari map memori
        if self.databases.remove(&db_key).is_none() {
            return Err(DomainError::DatabaseNotFound(db_name.into()));
        }

        // 2. Jika database yang dihapus sedang aktif, reset active_db
        if self
            .active_db
            .as_deref()
            .map_or(false, |active| active.eq_ignore_ascii_case(db_name))
        {
            self.active_db = None;
        }

        // 3. Hapus folder fisik di disk secara serentak (`Data_base/{username}/{dbname}`)
        let db_dir = format!("Data_base/{}/{}", username, db_name.to_lowercase());
        let path = Path::new(&db_dir);
        if path.exists() {
            std::fs::remove_dir_all(path).map_err(|e| {
                DomainError::storage(format!("Gagal menghapus folder database di disk: {e}"))
            })?;
        }

        Ok(())
    }
}

impl DatabaseManager {
    // ==========================================
    // USER MANAGEMENT DELEGATION
    // ==========================================

    pub fn create_user(&mut self, username: &str, password_hash: &str) -> Result<(), DomainError> {
        let current = self.current_username()?.to_lowercase();
        if !self.user_manager.is_admin(&current)? {
            return Err(DomainError::catalog(
                "Hanya admin yang dapat membuat user baru.",
            ));
        }
        self.user_manager.create_user(username, password_hash)?;
        self.save_users()
    }

    pub fn login(&mut self, username: &str, password: &str) -> Result<(), DomainError> {
        self.user_manager.authenticate(username, password)?;
        self.current_user = Some(username.to_string());
        self.active_db = None;
        Ok(())
    }

    pub fn change_password(
        &mut self,
        old_password: Option<&str>,
        new_password: &str,
    ) -> Result<(), DomainError> {
        let username = self.current_username()?.to_string();
        self.user_manager
            .change_password(&username, old_password, new_password)?;

        self.save_users()
    }
}

impl DatabaseManager {
    // ==========================================
    // EXECUTION FACADE
    // ==========================================

    pub fn execute(&mut self, action: CommandAction) -> Result<QueryResult, DomainError> {
        // Ambil referensi mutabel ke database aktif, lalu delegasikan eksekusi perintah ke `Database`
        let db = self.active_database_mut()?;
        db.execute(action)
    }
}
