use aes::cipher::{KeyIvInit, StreamCipher, SyncStreamCipher};
use aes::Aes256Ctr;
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use sha2::{Sha256, Digest};
use thiserror::Error;

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
pub struct SecureCryptoEngine {
    /// 主加密密钥
    master_key: [u8; 32],
    /// 密钥派生盐值
    salt: [u8; 32],
    /// 应用标识符
    app_identifier: [u8; 16],
}

impl SecureCryptoEngine {
    /// 创建新的加密引擎
    pub fn new(password: &str, salt: Option<&[u8]>) -> Result<Self, CryptoError> {
        // 生成或使用提供的盐值
        let mut salt_value = [0u8; 32];
        if let Some(s) = salt {
            if s.len() != 32 {
                return Err(CryptoError::KeyDerivationFailed);
            }
            salt_value.copy_from_slice(s);
        } else {
            rand::thread_rng().fill_bytes(&mut salt_value);
        }

        // 派生主密钥
        let mut master_key = [0u8; 32];
        pbkdf2_hmac::<Sha256>(
            password.as_bytes(),
            &salt_value,
            100_000,  // 高迭代次数防止暴力破解
            &mut master_key,
        );

        // 生成应用标识符
        let mut app_identifier = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut app_identifier);

        Ok(Self {
            master_key,
            salt: salt_value,
            app_identifier,
        })
    }

    /// 获取盐值
    pub fn get_salt(&self) -> &[u8; 32] {
        &self.salt
    }

    /// AES-256-CTR 加密
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        // 生成随机IV
        let mut iv = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut iv);

        // 创建加密器
        let cipher = Aes256Ctr::new_from_slice(&self.master_key, &iv);
        
        // 加密数据
        let mut ciphertext = plaintext.to_vec();
        cipher.apply_keystream(&mut ciphertext);

        // 组合 IV + 密文
        let mut result = Vec::new();
        result.extend_from_slice(&iv);
        result.extend_from_slice(&ciphertext);
        
        Ok(result)
    }

    /// AES-256-CTR 解密
    pub fn decrypt(&self, encrypted_data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if encrypted_data.len() < 16 {
            return Err(CryptoError::InvalidCiphertext);
        }

        // 分割 IV 和密文
        let (iv_bytes, ciphertext) = encrypted_data.split_at(16);
        
        // 创建解密器
        let cipher = Aes256Ctr::new_from_slice(&self.master_key, iv_bytes);
        
        // 解密数据
        let mut plaintext = ciphertext.to_vec();
        cipher.apply_keystream(&mut plaintext);

        Ok(plaintext)
    }

    /// AES-256-CTR 流加密（用于大文件）
    pub fn encrypt_ctr(&self, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let mut iv = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut iv);

        let cipher = Aes256Ctr::new_from_slice(&self.master_key, &iv);
        let mut ciphertext = plaintext.to_vec();
        cipher.apply_keystream(&mut ciphertext);

        let mut result = Vec::new();
        result.extend_from_slice(&iv);
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    /// AES-256-CTR 流解密
    pub fn decrypt_ctr(&self, encrypted_data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if encrypted_data.len() < 16 {
            return Err(CryptoError::InvalidCiphertext);
        }

        let (iv_bytes, ciphertext) = encrypted_data.split_at(16);
        
        let cipher = Aes256Ctr::new_from_slice(&self.master_key, iv_bytes);
        let mut plaintext = ciphertext.to_vec();
        cipher.apply_keystream(&mut plaintext);

        Ok(plaintext)
    }

    /// 生成安全的随机字符串
    pub fn generate_secure_random(length: usize) -> Result<String, CryptoError> {
        let mut buffer = vec![0u8; length];
        rand::thread_rng().fill_bytes(&mut buffer);
        
        // 使用安全的Base64编码
        Ok(base64::encode(buffer))
    }

    /// 计算SHA-256哈希
    pub fn sha256_hash(data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().into()
    }

    /// 验证数据完整性
    pub fn verify_integrity(&self, data: &[u8], expected_hash: &[u8; 32]) -> bool {
        let actual_hash = Self::sha256_hash(data);
        actual_hash == *expected_hash
    }

    /// 安全清除内存中的敏感数据
    pub fn secure_wipe(buffer: &mut [u8]) {
        for byte in buffer.iter_mut() {
            *byte = 0;
        }
    }
}