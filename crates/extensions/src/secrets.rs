//! API Key 不进 SQLite。优先系统密钥链，失败则落到应用数据目录的 0600 文件。

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use thiserror::Error;

pub const MODEL_API_KEY: &str = "model.api_key";

#[derive(Debug, Error)]
pub enum SecretError {
    #[error("invalid secret key: {0}")]
    InvalidKey(String),
    #[error("secret store unavailable: {0}")]
    Unavailable(String),
}

/// 注入内核服务表的密钥库。
pub struct SecretVault {
    memory: Mutex<HashMap<String, String>>,
    file_dir: Option<PathBuf>,
    use_keyring: bool,
}

impl SecretVault {
    pub fn memory() -> Self {
        Self {
            memory: Mutex::new(HashMap::new()),
            file_dir: None,
            use_keyring: false,
        }
    }

    pub fn open(data_dir: impl AsRef<Path>) -> Self {
        Self {
            memory: Mutex::new(HashMap::new()),
            file_dir: Some(data_dir.as_ref().join("secrets")),
            use_keyring: true,
        }
    }

    pub fn put(&self, key: &str, secret: &str) -> Result<(), SecretError> {
        validate_key(key)?;
        self.memory
            .lock()
            .map_err(|error| SecretError::Unavailable(error.to_string()))?
            .insert(key.to_owned(), secret.to_owned());

        if self.use_keyring {
            if let Ok(entry) = keyring::Entry::new("com.moshu.novel-agent", key) {
                let _ = entry.set_password(secret);
            }
        }
        if let Some(dir) = &self.file_dir {
            write_file_secret(dir, key, secret)?;
        }
        Ok(())
    }

    pub fn get(&self, key: &str) -> Result<Option<String>, SecretError> {
        validate_key(key)?;
        if let Some(value) = self
            .memory
            .lock()
            .map_err(|error| SecretError::Unavailable(error.to_string()))?
            .get(key)
            .cloned()
        {
            if !value.is_empty() {
                return Ok(Some(value));
            }
        }

        if self.use_keyring {
            if let Ok(entry) = keyring::Entry::new("com.moshu.novel-agent", key) {
                if let Ok(value) = entry.get_password() {
                    if !value.is_empty() {
                        self.memory
                            .lock()
                            .map_err(|error| SecretError::Unavailable(error.to_string()))?
                            .insert(key.to_owned(), value.clone());
                        return Ok(Some(value));
                    }
                }
            }
        }

        if let Some(dir) = &self.file_dir {
            if let Some(value) = read_file_secret(dir, key)? {
                self.memory
                    .lock()
                    .map_err(|error| SecretError::Unavailable(error.to_string()))?
                    .insert(key.to_owned(), value.clone());
                return Ok(Some(value));
            }
        }
        Ok(None)
    }

    pub fn delete(&self, key: &str) -> Result<(), SecretError> {
        validate_key(key)?;
        self.memory
            .lock()
            .map_err(|error| SecretError::Unavailable(error.to_string()))?
            .remove(key);
        if self.use_keyring {
            if let Ok(entry) = keyring::Entry::new("com.moshu.novel-agent", key) {
                let _ = entry.delete_credential();
            }
        }
        if let Some(dir) = &self.file_dir {
            let path = dir.join(key);
            if path.exists() {
                fs::remove_file(path)
                    .map_err(|error| SecretError::Unavailable(error.to_string()))?;
            }
        }
        Ok(())
    }

    pub fn is_set(&self, key: &str) -> bool {
        self.get(key)
            .ok()
            .flatten()
            .map(|value| !value.is_empty())
            .unwrap_or(false)
    }
}

fn validate_key(key: &str) -> Result<(), SecretError> {
    if key.is_empty()
        || !key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-')
    {
        return Err(SecretError::InvalidKey(key.into()));
    }
    Ok(())
}

fn write_file_secret(dir: &Path, key: &str, secret: &str) -> Result<(), SecretError> {
    fs::create_dir_all(dir).map_err(|error| SecretError::Unavailable(error.to_string()))?;
    let path = dir.join(key);
    fs::write(&path, secret).map_err(|error| SecretError::Unavailable(error.to_string()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
        let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o700));
    }
    Ok(())
}

fn read_file_secret(dir: &Path, key: &str) -> Result<Option<String>, SecretError> {
    let path = dir.join(key);
    if !path.exists() {
        return Ok(None);
    }
    let value =
        fs::read_to_string(path).map_err(|error| SecretError::Unavailable(error.to_string()))?;
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_roundtrip() {
        let vault = SecretVault::memory();
        vault.put(MODEL_API_KEY, "sk-test").unwrap();
        assert_eq!(
            vault.get(MODEL_API_KEY).unwrap().as_deref(),
            Some("sk-test")
        );
        assert!(vault.is_set(MODEL_API_KEY));
        vault.delete(MODEL_API_KEY).unwrap();
        assert!(!vault.is_set(MODEL_API_KEY));
    }

    #[test]
    fn file_fallback_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let vault = SecretVault::open(dir.path());
        vault.put(MODEL_API_KEY, "sk-file").unwrap();
        let other = SecretVault::open(dir.path());
        assert_eq!(
            other.get(MODEL_API_KEY).unwrap().as_deref(),
            Some("sk-file")
        );
    }

    #[test]
    fn rejects_path_key() {
        let vault = SecretVault::memory();
        assert!(vault.put("../etc/passwd", "x").is_err());
    }
}
