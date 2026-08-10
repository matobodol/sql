use crate::DomainError;
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

    pub fn create_user(&mut self, username: &str, password_hash: &str) -> Result<(), DomainError> {
        if self.users.contains_key(username) {
            return Err(DomainError::UserAlreadyExists(username.into()));
        }

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
