use aes_gcm::{
    aead::{Aead, KeyInit, generic_array::GenericArray},
    Aes256Gcm, Nonce,
};
use pbkdf2::pbkdf2;
use hmac::Hmac;
use sha2::{Sha256, Digest};
use rand::RngCore;
use base64::Engine;
use zeroize::Zeroize;
use thiserror::Error;

/// PBKDF2 迭代次数（OWASP 2023 对 PBKDF2-HMAC-SHA256 的建议下限为 600,000）。
/// 该值仅用于「新建」保险库；已有保险库的迭代次数从其持久化配置读取，
/// 因此提高此常量不会使旧数据无法解密。
pub const DEFAULT_PBKDF2_ITERATIONS: u32 = 600_000;
/// 主密钥派生盐值长度
const SALT_LEN: usize = 32;
/// GCM 推荐 nonce 长度
const NONCE_LEN: usize = 12;
/// GCM 认证标签长度
const TAG_LEN: usize = 16;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("加密失败")]
    EncryptionFailed,
    #[error("解密失败")]
    DecryptionFailed,
    #[error("密钥派生失败")]
    KeyDerivationFailed,
    #[error("无效的密文")]
    InvalidCiphertext,
    #[error("数据损坏")]
    DataCorrupted,
}

/// 高级加密引擎
///
/// 使用 AES-256-GCM（带认证加密），提供机密性与完整性/防篡改保护。
/// 密钥由主密码经 PBKDF2-HMAC-SHA256 派生；每次加密使用随机 nonce，
/// 密文格式为 nonce(12) 拼接 ciphertext（ciphertext 内含 16 字节 GCM 标签）。
#[derive(Clone)]
pub struct SecureCryptoEngine {
    /// 主加密密钥（32字节）
    master_key: [u8; 32],
    /// 密钥派生盐值（必须持久化，否则重启后无法恢复同一密钥解密历史数据）
    salt: [u8; SALT_LEN],
    /// 派生该密钥所用的 PBKDF2 迭代次数（持久化于配置，重启后按原值恢复）
    iterations: u32,
}

impl SecureCryptoEngine {
    /// 创建新的加密引擎
    ///
    /// - salt = None：生成随机盐值（用于首次设置）。
    /// - salt = Some(s)：使用提供的盐值（用于从已有状态恢复，必须与设置时一致）。
    pub fn new(password: &str, salt: Option<&[u8]>) -> Result<Self, CryptoError> {
        let mut salt_value = [0u8; SALT_LEN];
        match salt {
            Some(s) => {
                if s.len() != SALT_LEN {
                    return Err(CryptoError::KeyDerivationFailed);
                }
                salt_value.copy_from_slice(s);
            }
            None => {
                rand::thread_rng().fill_bytes(&mut salt_value);
            }
        }

        let iterations = DEFAULT_PBKDF2_ITERATIONS;
        let mut master_key = [0u8; 32];
        pbkdf2::<Hmac<Sha256>>(password.as_bytes(), &salt_value, iterations, &mut master_key);

        Ok(Self { master_key, salt: salt_value, iterations })
    }

    /// 获取盐值（需持久化）
    pub fn get_salt(&self) -> &[u8; SALT_LEN] {
        &self.salt
    }

    /// 按指定迭代次数构造引擎（用于按持久化配置恢复旧保险库）。
    pub fn new_with_iterations(
        password: &str,
        salt: Option<&[u8]>,
        iterations: u32,
    ) -> Result<Self, CryptoError> {
        let mut salt_value = [0u8; SALT_LEN];
        match salt {
            Some(s) => {
                if s.len() != SALT_LEN {
                    return Err(CryptoError::KeyDerivationFailed);
                }
                salt_value.copy_from_slice(s);
            }
            None => {
                rand::thread_rng().fill_bytes(&mut salt_value);
            }
        }

        let mut master_key = [0u8; 32];
        pbkdf2::<Hmac<Sha256>>(password.as_bytes(), &salt_value, iterations, &mut master_key);

        Ok(Self {
            master_key,
            salt: salt_value,
            iterations,
        })
    }

    /// 获取派生该密钥所用的迭代次数
    pub fn get_iterations(&self) -> u32 {
        self.iterations
    }

    /// AES-256-GCM 加密
    ///
    /// 返回 nonce 拼接 ciphertext（ciphertext 内含 GCM 认证标签）。
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let key = GenericArray::from_slice(&self.master_key);
        let cipher = Aes256Gcm::new(key);

        let mut nonce_bytes = [0u8; NONCE_LEN];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let mut ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|_| CryptoError::EncryptionFailed)?;

        let mut result = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.append(&mut ciphertext);
        Ok(result)
    }

    /// AES-256-GCM 解密
    ///
    /// 输入必须为 nonce(12) 拼接 ciphertext（含标签）。若密文被篡改或密码错误，
    /// GCM 认证失败将返回 DecryptionFailed，从而防止静默解密被篡改的数据。
    pub fn decrypt(&self, encrypted_data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if encrypted_data.len() < NONCE_LEN + TAG_LEN {
            return Err(CryptoError::InvalidCiphertext);
        }

        let (nonce_bytes, ciphertext) = encrypted_data.split_at(NONCE_LEN);
        let key = GenericArray::from_slice(&self.master_key);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(nonce_bytes);

        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| CryptoError::DecryptionFailed)
    }

    /// 生成安全的随机字符串（Base64 编码）
    pub fn generate_secure_random(length: usize) -> Result<String, CryptoError> {
        let mut buffer = vec![0u8; length];
        rand::thread_rng().fill_bytes(&mut buffer);
        Ok(base64::engine::general_purpose::STANDARD.encode(&buffer))
    }

    /// 计算 SHA-256 哈希
    pub fn sha256_hash(data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().into()
    }

    /// 验证数据完整性（比对 SHA-256 哈希）
    pub fn verify_integrity(&self, data: &[u8], expected_hash: &[u8; 32]) -> bool {
        let actual_hash = Self::sha256_hash(data);
        actual_hash == *expected_hash
    }

    /// 安全清除内存中的敏感数据
    pub fn secure_wipe(buffer: &mut [u8]) {
        buffer.zeroize();
    }
}

/// 引擎销毁时擦除主密钥，避免密钥在内存中残留。
/// （salt 不是秘密，但一起清掉没有副作用。）
impl Drop for SecureCryptoEngine {
    fn drop(&mut self) {
        self.master_key.zeroize();
        self.salt.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let engine = SecureCryptoEngine::new("StrongPass123!", None).unwrap();
        let ct = engine.encrypt(b"secret-data").unwrap();
        let pt = engine.decrypt(&ct).unwrap();
        assert_eq!(pt, b"secret-data");
    }

    #[test]
    fn tampered_ciphertext_rejected() {
        let engine = SecureCryptoEngine::new("StrongPass123!", None).unwrap();
        let mut ct = engine.encrypt(b"secret-data").unwrap();
        let last = ct.len() - 1;
        ct[last] ^= 0xff;
        assert!(engine.decrypt(&ct).is_err());
    }

    #[test]
    fn wrong_key_rejected() {
        let a = SecureCryptoEngine::new("StrongPass123!", None).unwrap();
        let salt = a.get_salt().to_vec();
        let iterations = a.get_iterations();
        let b = SecureCryptoEngine::new_with_iterations("WrongPass456!", Some(&salt), iterations).unwrap();
        let ct = a.encrypt(b"secret").unwrap();
        assert!(b.decrypt(&ct).is_err());
    }
}
