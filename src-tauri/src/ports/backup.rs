use std::path::Path;

use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupReceipt {
    pub format_version: u16,
    pub plaintext_sha256: String,
    pub encrypted_size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedRestore {
    pub staged_database_path: std::path::PathBuf,
    pub plaintext_sha256: String,
}

pub trait BackupService: Send + Sync {
    fn create_encrypted_backup(
        &self,
        database_path: &Path,
        destination: &Path,
        recovery_passphrase: &[u8],
    ) -> Result<BackupReceipt, AppError>;

    fn stage_restore(
        &self,
        backup_path: &Path,
        staging_directory: &Path,
        recovery_passphrase: &[u8],
    ) -> Result<StagedRestore, AppError>;

    fn cutover(&self, staged: &StagedRestore, destination: &Path) -> Result<(), AppError>;
}
