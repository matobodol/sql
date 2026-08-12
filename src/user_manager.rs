use crate::{BASE_PATH, DomainError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Permission {
    Read,
    Write,
    Admin,
    ManageDatabases,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub password_hash: String,
    pub global_permissions: Vec<Permission>,
    pub db_permissions: HashMap<String, Vec<Permission>>,
    pub password_changed: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct UserManager {
    users: HashMap<String, User>,
}

impl UserManager {
    // ==========================================
    // DISK MANAGEMENT
    // ==========================================
    pub fn load_or_new<P: AsRef<Path>>(base_path: P) -> Result<Self, DomainError> {
        let path = base_path.as_ref().join("users.json");
        if path.exists() {
            let data = fs::read_to_string(path)
                .map_err(|e| DomainError::catalog(format!("Gagal membaca file user: {e}")))?;
            let manager: UserManager = serde_json::from_str(&data)
                .map_err(|e| DomainError::catalog(format!("Gagal parsing data user: {e}")))?;
            Ok(manager)
        } else {
            let manager = Self::new();
            manager.save(&base_path)?;
            Ok(manager)
        }
    }

    pub fn save<P: AsRef<Path>>(&self, base_path: P) -> Result<(), DomainError> {
        let path = base_path.as_ref().join("users.json");
        let data = serde_json::to_string_pretty(self)
            .map_err(|e| DomainError::catalog(format!("Gagal serialisasi user: {e}")))?;
        fs::write(path, data)
            .map_err(|e| DomainError::catalog(format!("Gagal menyimpan file user: {e}")))?;
        Ok(())
    }
}

impl UserManager {
    // ==========================================
    // KONSTRUKTOR & HELPER
    // ==========================================
    pub fn new() -> Self {
        let mut manager = Self::default();
        manager.users.insert(
            "root".to_string(),
            User {
                username: "root".to_string(),
                password_hash: String::new(),
                global_permissions: vec![Permission::Admin],
                db_permissions: HashMap::new(),
                password_changed: false,
            },
        );
        manager
    }

    pub fn authenticate(&self, username: &str, password_hash: &str) -> Result<&User, DomainError> {
        let user = self
            .users
            .get(username)
            .ok_or_else(|| DomainError::UserNotFound(username.into()))?;

        // 1. Kasus Khusus Root Awal:
        // Jika root belum pernah ubah password, password awal memang String kosong ("")
        let is_initial_root = user.username == "root" && !user.password_changed;

        // 2. Pencocokan Password Normal:
        let is_password_valid = user.password_hash == password_hash;

        if is_initial_root || is_password_valid {
            Ok(user)
        } else {
            Err(DomainError::UserPasswordInvalid(username.into()))
        }
    }

    pub fn is_admin(&self, username: &str) -> Result<bool, DomainError> {
        let user = self
            .users
            .get(username)
            .ok_or_else(|| DomainError::UserNotFound(username.into()))?;
        Ok(user.global_permissions.contains(&Permission::Admin))
    }

    pub fn authorize(
        &self,
        username: &str,
        db_name: &str,
        required_perm: &Permission,
    ) -> Result<(), DomainError> {
        let user = self
            .users
            .get(username)
            .ok_or_else(|| DomainError::UserNotFound(username.into()))?;

        if user.global_permissions.contains(&Permission::Admin) {
            return Ok(());
        }

        if let Some(perms) = user.db_permissions.get(db_name) {
            if perms.contains(required_perm) || perms.contains(&Permission::Admin) {
                return Ok(());
            }
        }

        Err(DomainError::catalog(format!(
            "Access denied: user '{}' lacks permission '{:?}' on database '{}'",
            username, required_perm, db_name
        )))
    }
}

impl UserManager {
    // ==========================================
    // USER MANAGEMENT
    // ==========================================
    pub fn create_user(&mut self, username: &str, password_hash: &str) -> Result<(), DomainError> {
        if self.users.contains_key(username) {
            return Err(DomainError::UserAlreadyExists(username.into()));
        }

        // Buat folder fisik untuk user baru: data/{username}
        let user_dir = format!("{BASE_PATH}/{}", username.to_lowercase());
        std::fs::create_dir_all(&user_dir)
            .map_err(|e| DomainError::storage(format!("Gagal membuat direktori user: {e}")))?;

        let user = User {
            username: username.to_string(),
            password_hash: password_hash.to_string(),
            global_permissions: vec![Permission::Read, Permission::Write],
            db_permissions: HashMap::new(),
            password_changed: true,
        };

        self.users.insert(username.to_string(), user);
        Ok(())
    }

    pub fn rename_user(
        &mut self,
        old_username: &str,
        new_username: &str,
    ) -> Result<(), DomainError> {
        let old_lower = old_username.to_lowercase();
        let new_lower = new_username.to_lowercase();

        // 1. Pencegahan: User root tidak boleh di-rename
        if old_lower == "root" {
            return Err(DomainError::catalog("User root tidak dapat di-rename."));
        }

        // 2. Pastikan user baru belum ada
        if self.users.contains_key(new_username) {
            return Err(DomainError::UserAlreadyExists(new_username.into()));
        }

        // 3. Ambil dan ubah data user di dalam map memori (`self.users`)
        let mut user = self
            .users
            .remove(old_username)
            .ok_or_else(|| DomainError::UserNotFound(old_username.into()))?;

        user.username = new_username.to_string();
        self.users.insert(new_username.to_string(), user);

        // 4. Ubah nama folder fisik di disk secara serentak (`data/{old_name}` -> `{new_name}`)
        let old_dir = format!("{BASE_PATH}/{}", old_lower);
        let new_dir = format!("{BASE_PATH}/{}", new_lower);

        let old_path = Path::new(&old_dir);
        let new_path = Path::new(&new_dir);

        if old_path.exists() {
            std::fs::rename(old_path, new_path).map_err(|e| {
                DomainError::storage(format!("Gagal merename direktori user di disk: {e}"))
            })?;
        }

        Ok(())
    }

    pub fn drop_user(&mut self, username: &str) -> Result<(), DomainError> {
        let username_lower = username.to_lowercase();

        // Pencegahan agar user root tidak sengaja terhapus
        if username_lower == "root" {
            return Err(DomainError::catalog("User root tidak dapat dihapus."));
        }

        // 1. Hapus dari map memori (`self.users`)
        if self.users.remove(&username.to_lowercase()).is_none() {
            return Err(DomainError::UserNotFound(username.into()));
        }

        // 2. Hapus folder fisik user di disk secara serentak (`data/{username}`)
        let user_dir = format!("{BASE_PATH}/{}", username_lower);
        let path = Path::new(&user_dir);
        if path.exists() {
            std::fs::remove_dir_all(path).map_err(|e| {
                DomainError::storage(format!("Gagal menghapus direktori user di disk: {e}"))
            })?;
        }

        Ok(())
    }

    pub fn change_password(
        &mut self,
        username: &str,
        old_password_hash: Option<&str>,
        new_password_hash: &str,
    ) -> Result<(), DomainError> {
        let user = self
            .users
            .get_mut(username)
            .ok_or_else(|| DomainError::UserNotFound(username.into()))?;

        let is_initial_root = user.username == "root" && !user.password_changed;
        let is_old_pass_valid = old_password_hash.map_or(false, |old| user.password_hash == old);

        if is_initial_root || is_old_pass_valid {
            user.password_hash = new_password_hash.to_string();
            user.password_changed = true;
            Ok(())
        } else {
            Err(DomainError::UserPasswordInvalid(username.into()))
        }
    }
}
