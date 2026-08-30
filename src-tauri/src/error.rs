use serde::Serialize;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("authentication failed")]
    Authentication,
    #[error("authorization denied")]
    Authorization,
    #[error("entity was not found")]
    NotFound,
    #[error("stale entity revision")]
    StaleRevision,
    #[error("invalid workflow transition")]
    InvalidTransition,
    #[error("validation failed")]
    Validation,
    #[error("secret store unavailable")]
    SecretStoreUnavailable,
    #[error("secret was not found")]
    SecretNotFound,
    #[error("secret store access denied")]
    SecretAccessDenied,
    #[error("protected secret is corrupted")]
    SecretCorrupted,
    #[error("secret already exists")]
    SecretAlreadyExists,
    #[error("secret protection failed")]
    SecretProtectFailed,
    #[error("secret recovery failed")]
    SecretUnprotectFailed,
    #[error("database key is invalid")]
    DatabaseKeyInvalid,
    #[error("backup integrity validation failed")]
    BackupIntegrity,
    #[error("provider unavailable")]
    ProviderUnavailable,
    #[error("configuration rejected")]
    Configuration,
    #[error("persistence operation failed")]
    Persistence(#[from] rusqlite::Error),
    #[error("cryptographic operation failed")]
    Cryptography,
    #[error("input/output operation failed")]
    Io(#[from] std::io::Error),
    #[error("serialization failed")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: &'static str,
    pub correlation_id: Uuid,
}

impl From<AppError> for CommandError {
    fn from(value: AppError) -> Self {
        let code = match value {
            AppError::Authentication => "AUTHENTICATION_FAILED",
            AppError::Authorization => "AUTHORIZATION_DENIED",
            AppError::NotFound => "NOT_FOUND",
            AppError::StaleRevision => "STALE_REVISION",
            AppError::InvalidTransition => "INVALID_TRANSITION",
            AppError::Validation => "VALIDATION_FAILED",
            AppError::SecretStoreUnavailable => "SECRET_STORE_UNAVAILABLE",
            AppError::SecretNotFound => "SECRET_NOT_FOUND",
            AppError::SecretAccessDenied => "SECRET_ACCESS_DENIED",
            AppError::SecretCorrupted => "SECRET_CORRUPTED",
            AppError::SecretAlreadyExists => "SECRET_ALREADY_EXISTS",
            AppError::SecretProtectFailed => "SECRET_PROTECT_FAILED",
            AppError::SecretUnprotectFailed => "SECRET_UNPROTECT_FAILED",
            AppError::DatabaseKeyInvalid => "DATABASE_KEY_INVALID",
            AppError::BackupIntegrity => "BACKUP_INTEGRITY_FAILED",
            AppError::ProviderUnavailable => "PROVIDER_UNAVAILABLE",
            AppError::Configuration => "CONFIGURATION_REJECTED",
            AppError::Persistence(_) => "PERSISTENCE_FAILED",
            AppError::Cryptography => "CRYPTOGRAPHY_FAILED",
            AppError::Io(_) => "IO_FAILED",
            AppError::Serialization(_) => "SERIALIZATION_FAILED",
        };
        Self {
            code,
            correlation_id: Uuid::new_v4(),
        }
    }
}
