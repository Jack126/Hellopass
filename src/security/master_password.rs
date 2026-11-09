use crate::security::crypto::{SecureCryptoEngine, DEFAULT_PBKDF2_ITERATIONS};
use crate::security::key_management::KeyManager;
use serde::{Serialize, Deserialize};
use std::path::Path;
use std::fs::File;
use std::io::{Read, Write};
use base64::Engine;
use thiserror::Error;

/// 用于验证主密码的常量令牌：设置时用主密钥加密后持久化，验证时解密比对。
/// 这样既验证了密码正确性，也保证 salt 与主密码对应（GCM 认证会拒绝错误密码或被篡改数据）。
const VERIFY_TOKEN: &[u8] = b"SECUREPASS_MASTER_PASSWORD_VERIFY_V1";

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
    #[error("序列化错误")]
    SerdeError(#[from] serde_json::Error),
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

    /// 是否已设置主密码（依据配置文件是否存在）
    pub fn is_set(&self) -> bool {
        Path::new(&self.config_path).exists()
    }

    /// 设置主密码
    pub fn set_master_password(&mut self, password: &str) -> Result<(), MasterPasswordError> {
        if !self.validate_password_strength_inner(password) {
            return Err(MasterPasswordError::WeakPassword);
        }

        let key_manager = KeyManager::new(password, DEFAULT_PBKDF2_ITERATIONS)
            .map_err(|_| MasterPasswordError::StorageCorrupted)?;

        // 生成验证令牌密文（用当前主密钥加密 VERIFY_TOKEN），并持久化 salt + 令牌
        let engine = key_manager.get_crypto_engine();
        let salt = engine.get_salt().to_vec();
        let token_ct = engine.encrypt(VERIFY_TOKEN)
            .map_err(|_| MasterPasswordError::StorageCorrupted)?;

        let config = MasterPasswordConfig {
            created_at: chrono::Utc::now().to_rfc3339(),
            algorithm: "AES-256-GCM".to_string(),
            iterations: DEFAULT_PBKDF2_ITERATIONS,
            salt,
            verify_token: base64::engine::general_purpose::STANDARD.encode(&token_ct),
        };
        self.save_config(&config)?;

        self.key_manager = Some(key_manager);
        Ok(())
    }

    /// 验证主密码：用密码 + 持久化 salt 重建引擎并解密验证令牌。
    /// GCM 认证失败（密码错误或数据被篡改）将返回 IncorrectPassword。
    pub fn verify_password(&mut self, password: &str) -> Result<(), MasterPasswordError> {
        if !self.is_set() {
            return Err(MasterPasswordError::NotSet);
        }

        let config = self.load_config()?;

        let engine = SecureCryptoEngine::new_with_iterations(password, Some(config.salt.as_slice()), config.iterations)
            .map_err(|_| MasterPasswordError::IncorrectPassword)?;

        let token_ct = base64::engine::general_purpose::STANDARD.decode(&config.verify_token)
            .map_err(|_| MasterPasswordError::StorageCorrupted)?;
        let plaintext = engine.decrypt(&token_ct)
            .map_err(|_| MasterPasswordError::IncorrectPassword)?;

        if plaintext != VERIFY_TOKEN {
            return Err(MasterPasswordError::IncorrectPassword);
        }

        // 验证通过：构建正式密钥管理器供后续使用
        let key_manager = KeyManager::from_existing(password, config.salt.as_slice(), config.iterations)
            .map_err(|_| MasterPasswordError::IncorrectPassword)?;
        self.key_manager = Some(key_manager);
        Ok(())
    }

    pub fn get_crypto_engine(&self) -> Result<&SecureCryptoEngine, MasterPasswordError> {
        self.key_manager
            .as_ref()
            .map(|km| km.get_crypto_engine())
            .ok_or(MasterPasswordError::NotSet)
    }

    /// 导出加密数据
    pub fn export_encrypted_data(&self, data: &[u8]) -> Result<Vec<u8>, MasterPasswordError> {
        let crypto_engine = self.get_crypto_engine()?;
        let encrypted_data = crypto_engine.encrypt(data)
            .map_err(|_| MasterPasswordError::StorageCorrupted)?;
        let export_package = ExportPackage {
            version: "1.0".to_string(),
            algorithm: "AES-256-GCM".to_string(),
            salt: crypto_engine.get_salt().to_vec(),
            encrypted_data: base64::engine::general_purpose::STANDARD.encode(&encrypted_data),
            timestamp: chrono::Utc::now().to_rfc3339(),
            checksum: self.calculate_checksum(&encrypted_data),
        };
        Ok(serde_json::to_vec(&export_package)?)
    }

    /// 导入加密数据
    pub fn import_encrypted_data(&mut self, password: &str, export_data: &[u8]) -> Result<Vec<u8>, MasterPasswordError> {
        let export_package: ExportPackage = serde_json::from_slice(export_data)
            .map_err(|_| MasterPasswordError::StorageCorrupted)?;
        if export_package.version != "1.0" {
            return Err(MasterPasswordError::StorageCorrupted);
        }
        let encrypted_data = base64::engine::general_purpose::STANDARD.decode(&export_package.encrypted_data)
            .map_err(|_| MasterPasswordError::StorageCorrupted)?;
        if !self.verify_checksum(&encrypted_data, &export_package.checksum) {
            return Err(MasterPasswordError::StorageCorrupted);
        }
        let temp_crypto = SecureCryptoEngine::new_with_iterations(password, Some(export_package.salt.as_slice()), DEFAULT_PBKDF2_ITERATIONS)
            .map_err(|_| MasterPasswordError::IncorrectPassword)?;
        let decrypted_data = temp_crypto.decrypt(&encrypted_data)
            .map_err(|_| MasterPasswordError::IncorrectPassword)?;
        Ok(decrypted_data)
    }

    /// 修改主密码（会重新派生密钥，旧 salt/验证令牌被覆盖）
    pub fn change_password(&mut self, old_password: &str, new_password: &str) -> Result<(), MasterPasswordError> {
        self.verify_password(old_password)?;
        if !self.validate_password_strength_inner(new_password) {
            return Err(MasterPasswordError::WeakPassword);
        }
        self.set_master_password(new_password)?;
        Ok(())
    }

    pub fn delete_master_password(&mut self) -> Result<(), MasterPasswordError> {
        if Path::new(&self.config_path).exists() {
            std::fs::remove_file(&self.config_path)?;
        }
        self.key_manager = None;
        Ok(())
    }

    pub fn validate_password_strength_inner(&self, password: &str) -> bool {
        Self::password_is_strong(password)
    }

    /// 主密码准入规则（与 UI 提示保持一致）：≥12 字符且含大小写、数字、特殊字符。
    pub fn password_is_strong(password: &str) -> bool {
        if password.len() < 12 {
            return false;
        }
        let has_upper = password.chars().any(|c| c.is_uppercase());
        let has_lower = password.chars().any(|c| c.is_lowercase());
        let has_digit = password.chars().any(|c| c.is_digit(10));
        let has_special = password.chars().any(|c| !c.is_alphanumeric());
        has_upper && has_lower && has_digit && has_special
    }

    /// 原子写配置：先写同目录临时文件并 fsync，再 rename 覆盖。
    /// 这样任何时刻崩溃，磁盘上都只会是「完整旧配置」或「完整新配置」，
    /// 不会出现被截断的配置导致保险库永久无法打开。
    fn save_config(&self, config: &MasterPasswordConfig) -> Result<(), MasterPasswordError> {
        let config_json = serde_json::to_string(config)?;
        let path = Path::new(&self.config_path);
        let tmp_path = path.with_extension("json.tmp");
        {
            let mut file = File::create(&tmp_path)?;
            file.write_all(config_json.as_bytes())?;
            file.sync_all()?;
            // 配置文件含 salt 与主密码验证令牌，权限收紧为 0600
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(metadata) = std::fs::metadata(&tmp_path) {
                    let mut perms = metadata.permissions();
                    perms.set_mode(0o600);
                    let _ = std::fs::set_permissions(&tmp_path, perms);
                }
            }
        }
        std::fs::rename(&tmp_path, path)?;
        Ok(())
    }

    /// 用「已派生好的新密钥」替换主密码配置（改密流程使用）。
    ///
    /// 调用方负责先用旧密钥把数据重加密到新密钥；本方法只做最后一步的原子落盘，
    /// 因此落盘失败时磁盘上仍是旧配置，配合调用方的回滚即可回到一致状态。
    pub fn rotate_to(&mut self, engine: SecureCryptoEngine) -> Result<(), MasterPasswordError> {
        let token_ct = engine
            .encrypt(VERIFY_TOKEN)
            .map_err(|_| MasterPasswordError::StorageCorrupted)?;
        let config = MasterPasswordConfig {
            created_at: chrono::Utc::now().to_rfc3339(),
            algorithm: "AES-256-GCM".to_string(),
            iterations: engine.get_iterations(),
            salt: engine.get_salt().to_vec(),
            verify_token: base64::engine::general_purpose::STANDARD.encode(&token_ct),
        };
        self.save_config(&config)?;
        self.key_manager = Some(KeyManager::from_engine(engine));
        Ok(())
    }

    fn load_config(&self) -> Result<MasterPasswordConfig, MasterPasswordError> {
        let mut file = File::open(&self.config_path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        let config: MasterPasswordConfig = serde_json::from_str(&content)
            .map_err(|_| MasterPasswordError::StorageCorrupted)?;
        Ok(config)
    }

    fn calculate_checksum(&self, data: &[u8]) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    fn verify_checksum(&self, data: &[u8], expected_checksum: &str) -> bool {
        self.calculate_checksum(data) == expected_checksum
    }
}

#[derive(Serialize, Deserialize)]
struct MasterPasswordConfig {
    created_at: String,
    algorithm: String,
    iterations: u32,
    salt: Vec<u8>,
    verify_token: String,
}

#[derive(Serialize, Deserialize)]
struct ExportPackage {
    version: String,
    algorithm: String,
    salt: Vec<u8>,
    encrypted_data: String,
    timestamp: String,
    checksum: String,
}
