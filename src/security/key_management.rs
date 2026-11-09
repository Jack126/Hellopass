use crate::security::crypto::SecureCryptoEngine;
use serde::{Serialize, Deserialize};
use std::path::Path;
use std::fs;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KeyManagementError {
    #[error("密钥管理器未初始化")]
    NotInitialized,
    #[error("密钥派生失败")]
    KeyDerivationFailed,
    #[error("存储文件损坏")]
    StorageCorrupted,
    #[error("IO错误")]
    IoError(#[from] std::io::Error),
    #[error("序列化错误")]
    SerdeError(#[from] serde_json::Error),
}

/// 密钥管理器
pub struct KeyManager {
    crypto_engine: SecureCryptoEngine,
    created_at: String,
}

impl KeyManager {
    /// 创建新的密钥管理器（随机生成 salt）
    pub fn new(password: &str, iterations: u32) -> Result<Self, KeyManagementError> {
        let crypto_engine = SecureCryptoEngine::new_with_iterations(password, None, iterations)
            .map_err(|_| KeyManagementError::KeyDerivationFailed)?;
        let created_at = chrono::Utc::now().to_rfc3339();
        Ok(Self { crypto_engine, created_at })
    }

    /// 从已有 salt 恢复密钥管理器（用于验证主密码 / 重启恢复）
    pub fn from_existing(password: &str, salt: &[u8], iterations: u32) -> Result<Self, KeyManagementError> {
        let crypto_engine = SecureCryptoEngine::new_with_iterations(password, Some(salt), iterations)
            .map_err(|_| KeyManagementError::KeyDerivationFailed)?;
        let created_at = chrono::Utc::now().to_rfc3339();
        Ok(Self { crypto_engine, created_at })
    }

    pub fn get_crypto_engine(&self) -> &SecureCryptoEngine {
        &self.crypto_engine
    }

    pub fn get_created_at(&self) -> &str {
        &self.created_at
    }

    /// 保存密钥管理器状态（含 salt，供重启恢复）
    pub fn save_state(&self, config_dir: &str) -> Result<(), KeyManagementError> {
        let config_path = Path::new(config_dir).join("key_manager_state.json");
        let config = KeyManagerState {
            created_at: self.created_at.clone(),
            salt: self.crypto_engine.get_salt().to_vec(),
        };
        let config_json = serde_json::to_string(&config)?;
        fs::write(config_path, config_json)?;
        Ok(())
    }

    /// 加载密钥管理器状态（用密码 + 持久化 salt 恢复同一密钥）
    pub fn load_state(config_dir: &str, password: &str) -> Result<Self, KeyManagementError> {
        let config_path = Path::new(config_dir).join("key_manager_state.json");
        if !config_path.exists() {
            return Err(KeyManagementError::NotInitialized);
        }
        let config_content = fs::read_to_string(&config_path)?;
        let config: KeyManagerState = serde_json::from_str(&config_content)
            .map_err(|_| KeyManagementError::StorageCorrupted)?;
        let crypto_engine = SecureCryptoEngine::new(password, Some(config.salt.as_slice()))
            .map_err(|_| KeyManagementError::KeyDerivationFailed)?;
        Ok(Self { crypto_engine, created_at: config.created_at })
    }
}

/// 密钥管理器状态（持久化 salt）
#[derive(Serialize, Deserialize)]
pub struct KeyManagerState {
    created_at: String,
    salt: Vec<u8>,
}
