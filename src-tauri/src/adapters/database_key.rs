use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zeroize::Zeroizing;

#[cfg(feature = "sqlcipher")]
use crate::adapters::Database;
#[cfg(feature = "sqlcipher")]
use crate::adapters::generate_database_key;
use crate::error::AppError;
use crate::ports::SecretStore;

const DESCRIPTOR_VERSION: u16 = 1;
const MAX_DESCRIPTOR_BYTES: u64 = 8 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DatabaseKeyDescriptor {
    pub descriptor_version: u16,
    pub database_id: Uuid,
    pub key_reference: String,
}

pub struct RecoveredDatabaseKey {
    pub descriptor: DatabaseKeyDescriptor,
    pub key: Zeroizing<Vec<u8>>,
}

pub struct DatabaseKeyLifecycle<'a> {
    secret_store: &'a dyn SecretStore,
}

impl<'a> DatabaseKeyLifecycle<'a> {
    pub fn new(secret_store: &'a dyn SecretStore) -> Self {
        Self { secret_store }
    }

    pub fn descriptor_path(database_path: &Path) -> PathBuf {
        let mut filename = database_path
            .file_name()
            .map(OsString::from)
            .unwrap_or_else(|| OsString::from("autovaxx.sqlite"));
        filename.push(".keyref.json");
        database_path.with_file_name(filename)
    }

    pub fn recover(&self, database_path: &Path) -> Result<RecoveredDatabaseKey, AppError> {
        if !database_path.is_file() {
            return Err(AppError::NotFound);
        }
        let descriptor = Self::read_descriptor(&Self::descriptor_path(database_path))?;
        Self::validate_descriptor(&descriptor)?;
        let key = Zeroizing::new(self.secret_store.load(&descriptor.key_reference)?);
        if key.len() != 32 {
            return Err(AppError::SecretCorrupted);
        }
        Ok(RecoveredDatabaseKey { descriptor, key })
    }

    pub fn adopt_encrypted_database(
        &self,
        staged_database_path: &Path,
        destination: &Path,
        key: &[u8],
    ) -> Result<DatabaseKeyDescriptor, AppError> {
        let descriptor_path = Self::descriptor_path(destination);
        if destination.exists() || descriptor_path.exists() || !staged_database_path.is_file() {
            return Err(AppError::Validation);
        }
        if key.len() != 32 {
            return Err(AppError::SecretCorrupted);
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let database_id = Uuid::new_v4();
        let descriptor = DatabaseKeyDescriptor {
            descriptor_version: DESCRIPTOR_VERSION,
            database_id,
            key_reference: format!("autovaxx-db-{database_id}"),
        };
        self.secret_store.store(&descriptor.key_reference, key)?;
        if let Err(error) = Self::write_descriptor(&descriptor_path, &descriptor) {
            let _ = self.secret_store.delete(&descriptor.key_reference);
            return Err(error);
        }
        if let Err(error) = fs::rename(staged_database_path, destination) {
            let _ = fs::remove_file(&descriptor_path);
            let _ = self.secret_store.delete(&descriptor.key_reference);
            return Err(AppError::Io(error));
        }
        Ok(descriptor)
    }

    #[cfg(feature = "sqlcipher")]
    pub fn create_encrypted_database(&self, database_path: &Path) -> Result<Database, AppError> {
        let descriptor_path = Self::descriptor_path(database_path);
        if database_path.exists() || descriptor_path.exists() {
            return Err(AppError::Validation);
        }

        let database_id = Uuid::new_v4();
        let descriptor = DatabaseKeyDescriptor {
            descriptor_version: DESCRIPTOR_VERSION,
            database_id,
            key_reference: format!("autovaxx-db-{database_id}"),
        };
        let key = generate_database_key()?;
        self.secret_store.store(&descriptor.key_reference, &key)?;

        let database = match Database::create_encrypted(database_path, &key) {
            Ok(database) => database,
            Err(error) => {
                let _ = self.secret_store.delete(&descriptor.key_reference);
                Self::remove_new_database_artifacts(database_path);
                return Err(error);
            }
        };
        if let Err(error) = Self::write_descriptor(&descriptor_path, &descriptor) {
            drop(database);
            let _ = self.secret_store.delete(&descriptor.key_reference);
            Self::remove_new_database_artifacts(database_path);
            return Err(error);
        }
        Ok(database)
    }

    #[cfg(feature = "sqlcipher")]
    pub fn open_encrypted_database(&self, database_path: &Path) -> Result<Database, AppError> {
        let recovered = self.recover(database_path)?;
        Database::open_encrypted(database_path, &recovered.key)
    }

    fn read_descriptor(path: &Path) -> Result<DatabaseKeyDescriptor, AppError> {
        let metadata = fs::metadata(path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                AppError::SecretNotFound
            } else {
                AppError::Io(error)
            }
        })?;
        if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_DESCRIPTOR_BYTES {
            return Err(AppError::SecretCorrupted);
        }
        let file = fs::File::open(path)?;
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        file.take(MAX_DESCRIPTOR_BYTES + 1)
            .read_to_end(&mut bytes)?;
        serde_json::from_slice(&bytes).map_err(|_| AppError::SecretCorrupted)
    }

    fn write_descriptor(path: &Path, descriptor: &DatabaseKeyDescriptor) -> Result<(), AppError> {
        let bytes = serde_json::to_vec(descriptor)?;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        Ok(())
    }

    fn validate_descriptor(descriptor: &DatabaseKeyDescriptor) -> Result<(), AppError> {
        if descriptor.descriptor_version != DESCRIPTOR_VERSION
            || descriptor.key_reference != format!("autovaxx-db-{}", descriptor.database_id)
        {
            return Err(AppError::SecretCorrupted);
        }
        Ok(())
    }

    #[cfg(feature = "sqlcipher")]
    fn remove_new_database_artifacts(database_path: &Path) {
        let _ = fs::remove_file(database_path);
        for suffix in ["-wal", "-shm", "-journal"] {
            let mut artifact = database_path.as_os_str().to_os_string();
            artifact.push(suffix);
            let _ = fs::remove_file(PathBuf::from(artifact));
        }
    }
}

#[cfg(all(test, feature = "sqlcipher"))]
mod tests {
    use super::*;
    use crate::adapters::{FakeSecretStore, SecretStoreFault};

    #[test]
    fn encrypted_database_key_survives_restart_without_raw_key_on_disk() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("integrated.sqlite");
        let store = FakeSecretStore::new();
        let lifecycle = DatabaseKeyLifecycle::new(&store);
        drop(lifecycle.create_encrypted_database(&path).unwrap());

        let protected_key = lifecycle.recover(&path).unwrap().key;
        let descriptor_bytes = fs::read(DatabaseKeyLifecycle::descriptor_path(&path)).unwrap();
        assert!(
            !descriptor_bytes
                .windows(protected_key.len())
                .any(|window| window == protected_key.as_slice())
        );
        let raw_database = fs::read(&path).unwrap();
        assert!(!raw_database.starts_with(b"SQLite format 3\0"));
        assert!(lifecycle.open_encrypted_database(&path).is_ok());
    }

    #[test]
    fn missing_corrupt_denied_and_wrong_protected_keys_fail_closed() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("fail-closed.sqlite");
        let store = FakeSecretStore::new();
        let lifecycle = DatabaseKeyLifecycle::new(&store);
        drop(lifecycle.create_encrypted_database(&path).unwrap());
        let descriptor = lifecycle.recover(&path).unwrap().descriptor;

        store.delete(&descriptor.key_reference).unwrap();
        assert!(matches!(
            lifecycle.open_encrypted_database(&path),
            Err(AppError::SecretNotFound)
        ));
        store.store(&descriptor.key_reference, &[7; 31]).unwrap();
        assert!(matches!(
            lifecycle.open_encrypted_database(&path),
            Err(AppError::SecretCorrupted)
        ));
        store.replace_for_test(&descriptor.key_reference, &[9; 32]);
        assert!(matches!(
            lifecycle.open_encrypted_database(&path),
            Err(AppError::DatabaseKeyInvalid)
        ));
        store.inject_fault(Some(SecretStoreFault::AccessDenied));
        assert!(matches!(
            lifecycle.open_encrypted_database(&path),
            Err(AppError::SecretAccessDenied)
        ));
        store.inject_fault(Some(SecretStoreFault::Corrupted));
        assert!(matches!(
            lifecycle.open_encrypted_database(&path),
            Err(AppError::SecretCorrupted)
        ));
        store.inject_fault(Some(SecretStoreFault::Unavailable));
        assert!(matches!(
            lifecycle.open_encrypted_database(&path),
            Err(AppError::SecretStoreUnavailable)
        ));
    }

    #[test]
    fn copied_database_or_key_reference_alone_cannot_open() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.sqlite");
        let store = FakeSecretStore::new();
        let lifecycle = DatabaseKeyLifecycle::new(&store);
        drop(lifecycle.create_encrypted_database(&source).unwrap());

        let database_only = temp.path().join("database-only.sqlite");
        fs::copy(&source, &database_only).unwrap();
        assert!(matches!(
            lifecycle.open_encrypted_database(&database_only),
            Err(AppError::SecretNotFound)
        ));

        let reference_only = temp.path().join("reference-only.sqlite");
        fs::copy(
            DatabaseKeyLifecycle::descriptor_path(&source),
            DatabaseKeyLifecycle::descriptor_path(&reference_only),
        )
        .unwrap();
        assert!(matches!(
            lifecycle.open_encrypted_database(&reference_only),
            Err(AppError::NotFound)
        ));
    }

    #[test]
    fn failed_key_protection_leaves_no_database_or_descriptor() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("protect-failure.sqlite");
        let store = FakeSecretStore::new();
        store.inject_fault(Some(SecretStoreFault::ProtectFailed));
        let lifecycle = DatabaseKeyLifecycle::new(&store);
        assert!(matches!(
            lifecycle.create_encrypted_database(&path),
            Err(AppError::SecretProtectFailed)
        ));
        assert!(!path.exists());
        assert!(!DatabaseKeyLifecycle::descriptor_path(&path).exists());
    }
}
