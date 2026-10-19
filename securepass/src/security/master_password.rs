use crate::security::crypto::SecureCryptoEngine;
use crate::security::key_management::KeyManager;
use serde::{Serialize, Deserialize};
use std::path::Path;
use std::fs::File;
use std::io::{Read, Write};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MasterPasswordError {
    #[error("主密码未设置")]
    NotSet,
    #[error("主密码错误")]
    IncorrectPassword,
    #[error("主密码强度不足")]
    WeakPassword,
    #[error("存储文件损坏")]
    StorageCorrupted,
    #[error("IO错误")]
    IoError(#[from] std::io::Error),
}

/// 主密码管理器
pub struct MasterPasswordManager {
    key_manager: Option<KeyManager>,
    config_path: String,
}

impl MasterPasswordManager {
    pub fn new(config_dir: &str) -> Self {
        let config_path = Path::new(config_dir).join("master_password_config.json");
        
        Self {
            key_manager: None,
            config_path: config_path.to_string_lossy().to_string(),
        }
    }

    /// 检查是否已设置主密码
    pub fn is_set(&self) -> bool {
        self.key_manager.is_some()
    }

    /// 设置主密码
    pub fn set_master_password(&mut self, password: &str) -> Result<(), MasterPasswordError> {
        // 验证密码强度
        if !self.validate_password_strength(password) {
            return Err(MasterPasswordError::WeakPassword);
        }

        // 创建新的密钥管理器
        let key_manager = KeyManager::new(password)
            .map_err(|_| MasterPasswordError::StorageCorrupted)?;
        
        // 保存配置
        self.save_config(password)?;
        
        self.key_manager = Some(key_manager);
        Ok(())
    }

    /// 验证主密码
    pub fn verify_password(&mut self, password: &str) -> Result<(), MasterPasswordError> {
        if !self.is_set() {
            return Err(MasterPasswordError::NotSet);
        }

        // 尝试加载配置
        let config = self.load_config()?;
        
        // 尝试创建密钥管理器
        let key_manager = KeyManager::from_existing(password, config)
            .map_err(|_| MasterPasswordError::IncorrectPassword)?;
        
        self.key_manager = Some(key_manager);
        Ok(())
    }

    /// 获取加密引擎
    pub fn get_crypto_engine(&self) -> Result<&SecureCryptoEngine, MasterPasswordError> {
        self.key_manager
            .as_ref()
            .map(|km| km.get_crypto_engine())
            .ok_or(MasterPasswordError::NotSet)
    }

    /// 导出加密数据
    pub fn export_encrypted_data(&self, data: &[u8]) -> Result<Vec<u8>, MasterPasswordError> {
        let crypto_engine = self.get_crypto_engine()?;
        
        // 加密数据
        let encrypted_data = crypto_engine.encrypt_gcm(data)
            .map_err(|_| MasterPasswordError::StorageCorrupted)?;
        
        // 创建导出包
        let export_package = ExportPackage {
            version: "1.0".to_string(),
            algorithm: "AES-256-GCM".to_string(),
            salt: crypto_engine.get_salt().to_vec(),
            encrypted_data: base64::encode(encrypted_data),
            timestamp: chrono::Utc::now().to_rfc3339(),
            checksum: self.calculate_checksum(&encrypted_data),
        };
        
        Ok(serde_json::to_vec(&export_package)?)
    }

    /// 导入加密数据
    pub fn import_encrypted_data(&mut self, password: &str, export_data: &[u8]) -> Result<Vec<u8>, MasterPasswordError> {
        // 解析导出包
        let export_package: ExportPackage = serde_json::from_slice(export_data)
            .map_err(|_| MasterPasswordError::StorageCorrupted)?;
        
        // 验证导出包
        if export_package.version != "1.0" {
            return Err(MasterPasswordError::StorageCorrupted);
        }
        
        // 验证校验和
        let encrypted_data = base64::decode(&export_package.encrypted_data)
            .map_err(|_| MasterPasswordError::StorageCorrupted)?;
        
        if !self.verify_checksum(&encrypted_data, &export_package.checksum) {
            return Err(MasterPasswordError::StorageCorrupted);
        }
        
        // 使用主密码创建临时加密引擎
        let temp_crypto = SecureCryptoEngine::new(password, Some(&export_package.salt))
            .map_err(|_| MasterPasswordError::IncorrectPassword)?;
        
        // 解密数据
        let decrypted_data = temp_crypto.decrypt_gcm(&encrypted_data)
            .map_err(|_| MasterPasswordError::IncorrectPassword)?;
        
        Ok(decrypted_data)
    }

    /// 修改主密码
    pub fn change_password(&mut self, old_password: &str, new_password: &str) -> Result<(), MasterPasswordError> {
        // 验证旧密码
        self.verify_password(old_password)?;
        
        // 验证新密码强度
        if !self.validate_password_strength(new_password) {
            return Err(MasterPasswordError::WeakPassword);
        }
        
        // 重新设置主密码
        self.set_master_password(new_password)?;
        Ok(())
    }

    /// 删除主密码（删除所有数据）
    pub fn delete_master_password(&mut self) -> Result<(), MasterPasswordError> {
        // 删除配置文件
        if std::path::Path::new(&self.config_path).exists() {
            std::fs::remove_file(&self.config_path)?;
        }
        
        self.key_manager = None;
        Ok(())
    }

    /// 验证密码强度
    fn validate_password_strength(&self, password: &str) -> bool {
        // 最小长度 12
        if password.len() < 12 {
            return false;
        }
        
        // 必须包含大小写字母、数字、特殊字符
        let has_upper = password.chars().any(|c| c.is_uppercase());
        let has_lower = password.chars().any(|c| c.is_lowercase());
        let has_digit = password.chars().any(|c| c.is_digit(10));
        let has_special = password.chars().any(|c| !c.is_alphanumeric());
        
        has_upper && has_lower && has_digit && has_special
    }

    /// 保存配置
    fn save_config(&self, password: &str) -> Result<(), MasterPasswordError> {
        let config = MasterPasswordConfig {
            created_at: chrono::Utc::now().to_rfc3339(),
            algorithm: "PBKDF2-HMAC-SHA256".to_string(),
            iterations: 100_000,
            salt_length: 32,
        };
        
        let config_json = serde_json::to_string(&config)?;
        let mut file = File::create(&self.config_path)?;
        file.write_all(config_json.as_bytes())?;
        
        Ok(())
    }

    /// 加载配置
    fn load_config(&self) -> Result<MasterPasswordConfig, MasterPasswordError> {
        let mut file = File::open(&self.config_path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        
        let config: MasterPasswordConfig = serde_json::from_str(&content)
            .map_err(|_| MasterPasswordError::StorageCorrupted)?;
        
        Ok(config)
    }

    /// 计算校验和
    fn calculate_checksum(&self, data: &[u8]) -> String {
        use sha2::{Sha256, Digest};
        let hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    /// 验证校验和
    fn verify_checksum(&self, data: &[u8], expected_checksum: &str) -> bool {
        let actual_checksum = self.calculate_checksum(data);
        actual_checksum == expected_checksum
    }
}

/// 主密码配置
#[derive(Serialize, Deserialize)]
struct MasterPasswordConfig {
    created_at: String,
    algorithm: String,
    iterations: u32,
    salt_length: usize,
}

/// 导出包结构
#[derive(Serialize, Deserialize)]
struct ExportPackage {
    version: String,
    algorithm: String,
    salt: Vec<u8>,
    encrypted_data: String,
    timestamp: String,
    checksum: String,
}