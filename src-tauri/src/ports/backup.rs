use std::fmt;
use std::path::{Path, PathBuf};

use uuid::Uuid;
use zeroize::Zeroizing;

use crate::error::AppError;
use crate::ports::SecretStore;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupReceipt {
    pub backup_id: Uuid,
    pub format_version: u16,
    pub snapshot_sha256: String,
    pub encrypted_size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreSummary {
    pub schema_version: u32,
    pub audit_event_count: u64,
    pub patient_count: u64,
    pub encounter_count: u64,
}

pub struct StagedRestore {
    pub backup_id: Uuid,
    pub staged_database_path: PathBuf,
    pub snapshot_sha256: String,
    pub summary: RestoreSummary,
    #[cfg_attr(not(feature = "sqlcipher"), allow(dead_code))]
    pub(crate) snapshot_key: Zeroizing<Vec<u8>>,
}

impl fmt::Debug for StagedRestore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StagedRestore")
            .field("backup_id", &self.backup_id)
            .field("staged_database_path", &self.staged_database_path)
            .field("snapshot_sha256", &self.snapshot_sha256)
            .field("summary", &self.summary)
            .field("snapshot_key", &"[REDACTED]")
            .finish()
    }
}

impl Drop for StagedRestore {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.staged_database_path);
    }
}

pub trait EncryptedSnapshotSource: Send + Sync {
    fn write_encrypted_snapshot(
        &self,
        destination: &Path,
        snapshot_key: &[u8],
    ) -> Result<RestoreSummary, AppError>;
}

pub trait BackupService: Send + Sync {
    fn create_encrypted_backup(
        &self,
        source: &dyn EncryptedSnapshotSource,
        destination: &Path,
        recovery_secret: &[u8],
    ) -> Result<BackupReceipt, AppError>;

    fn stage_restore(
        &self,
        backup_path: &Path,
        staging_directory: &Path,
        recovery_secret: &[u8],
    ) -> Result<StagedRestore, AppError>;

    fn cutover(
        &self,
        staged: StagedRestore,
        destination: &Path,
        secret_store: &dyn SecretStore,
    ) -> Result<(), AppError>;
}
