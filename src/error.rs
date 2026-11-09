//! 跨模块统一错误类型：把 database / security 各层错误收敛为 AppError，
//! 便于在边界统一处理与向上传递（`?` 自动转换）。

use crate::database::DatabaseError;
use crate::database::operations::OperationError;
use crate::security::crypto::CryptoError;
use crate::security::master_password::MasterPasswordError;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error(transparent)]
    Operation(#[from] OperationError),
    #[error(transparent)]
    Crypto(#[from] CryptoError),
    #[error(transparent)]
    MasterPassword(#[from] MasterPasswordError),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error("{0}")]
    Message(String),
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_domain_errors() {
        let e: AppError = OperationError::NotFound.into();
        assert!(matches!(e, AppError::Operation(_)));
        let e2: AppError = CryptoError::DataCorrupted.into();
        assert!(matches!(e2, AppError::Crypto(_)));
        let e3 = AppError::Message(String::new());
        assert!(e3.to_string().is_empty());
    }
}
