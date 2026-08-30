use crate::error::AppError;

pub trait SecretStore: Send + Sync {
    fn store(&self, key_reference: &str, secret: &[u8]) -> Result<(), AppError>;
    fn load(&self, key_reference: &str) -> Result<Vec<u8>, AppError>;
    fn delete(&self, key_reference: &str) -> Result<(), AppError>;
}
