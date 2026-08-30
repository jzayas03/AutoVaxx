use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::Duration;

use aes_gcm::aead::{Aead, Payload};
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::Engine;
use base64::engine::general_purpose::STANDARD_NO_PAD;
use chrono::Utc;
use rusqlite::{Connection, OpenFlags, backup::Backup};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

use crate::error::AppError;
use crate::ports::{BackupReceipt, BackupService, StagedRestore};

const MAGIC: &[u8; 8] = b"AVXBAK01";
const FORMAT_VERSION: u16 = 1;
const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_BACKUP_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const KDF_MEMORY_KIB: u32 = 65_536;
const KDF_ITERATIONS: u32 = 3;
const KDF_PARALLELISM: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupHeader {
    format: String,
    format_version: u16,
    created_at_utc: String,
    cipher: String,
    kdf: String,
    kdf_memory_kib: u32,
    kdf_iterations: u32,
    kdf_parallelism: u32,
    salt_b64: String,
    wrapped_key_nonce_b64: String,
    wrapped_content_key_b64: String,
    payload_nonce_b64: String,
    plaintext_sha256: String,
    data_classification: String,
}

#[derive(Default)]
pub struct EncryptedBackupService;

impl EncryptedBackupService {
    fn random<const N: usize>() -> Result<[u8; N], AppError> {
        let mut value = [0_u8; N];
        getrandom::fill(&mut value).map_err(|_| AppError::Cryptography)?;
        Ok(value)
    }

    fn derive_wrapping_key(
        passphrase: &[u8],
        salt: &[u8],
    ) -> Result<Zeroizing<[u8; 32]>, AppError> {
        if passphrase.len() < 16 {
            return Err(AppError::Validation);
        }
        let params = Params::new(KDF_MEMORY_KIB, KDF_ITERATIONS, KDF_PARALLELISM, Some(32))
            .map_err(|_| AppError::Cryptography)?;
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        let mut key = Zeroizing::new([0_u8; 32]);
        argon2
            .hash_password_into(passphrase, salt, key.as_mut())
            .map_err(|_| AppError::Cryptography)?;
        Ok(key)
    }

    fn sqlite_snapshot(
        database_path: &Path,
        destination_directory: &Path,
    ) -> Result<NamedTempFile, AppError> {
        fs::create_dir_all(destination_directory)?;
        let source = Connection::open_with_flags(database_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let check: String = source.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
        if check != "ok" {
            return Err(AppError::BackupIntegrity);
        }
        let snapshot = NamedTempFile::new_in(destination_directory)?;
        let mut destination = Connection::open(snapshot.path())?;
        let backup = Backup::new(&source, &mut destination)?;
        backup.run_to_completion(5, Duration::from_millis(10), None)?;
        drop(backup);
        destination.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        drop(destination);
        Ok(snapshot)
    }

    fn encode(
        database_bytes: &[u8],
        recovery_passphrase: &[u8],
    ) -> Result<(Vec<u8>, BackupHeader), AppError> {
        let salt = Self::random::<16>()?;
        let wrap_nonce = Self::random::<12>()?;
        let payload_nonce = Self::random::<12>()?;
        let mut content_key = Zeroizing::new(Self::random::<32>()?);
        let wrapping_key = Self::derive_wrapping_key(recovery_passphrase, &salt)?;
        let wrap_cipher =
            Aes256Gcm::new_from_slice(wrapping_key.as_ref()).map_err(|_| AppError::Cryptography)?;
        let mut wrap_aad = Vec::from(MAGIC.as_slice());
        wrap_aad.extend_from_slice(&FORMAT_VERSION.to_be_bytes());
        wrap_aad.extend_from_slice(&salt);
        let wrapped_content_key = wrap_cipher
            .encrypt(
                Nonce::from_slice(&wrap_nonce),
                Payload {
                    msg: content_key.as_ref(),
                    aad: &wrap_aad,
                },
            )
            .map_err(|_| AppError::Cryptography)?;

        let plaintext_sha256 = hex_sha256(database_bytes);
        let header = BackupHeader {
            format: "AUTOVAXX_ENCRYPTED_SQLITE".to_owned(),
            format_version: FORMAT_VERSION,
            created_at_utc: Utc::now().to_rfc3339(),
            cipher: "AES-256-GCM".to_owned(),
            kdf: "ARGON2ID-1.3".to_owned(),
            kdf_memory_kib: KDF_MEMORY_KIB,
            kdf_iterations: KDF_ITERATIONS,
            kdf_parallelism: KDF_PARALLELISM,
            salt_b64: STANDARD_NO_PAD.encode(salt),
            wrapped_key_nonce_b64: STANDARD_NO_PAD.encode(wrap_nonce),
            wrapped_content_key_b64: STANDARD_NO_PAD.encode(wrapped_content_key),
            payload_nonce_b64: STANDARD_NO_PAD.encode(payload_nonce),
            plaintext_sha256,
            data_classification: "SYNTHETIC_ONLY".to_owned(),
        };
        let header_bytes = serde_json::to_vec(&header)?;
        let content_cipher =
            Aes256Gcm::new_from_slice(content_key.as_ref()).map_err(|_| AppError::Cryptography)?;
        let ciphertext = content_cipher
            .encrypt(
                Nonce::from_slice(&payload_nonce),
                Payload {
                    msg: database_bytes,
                    aad: &header_bytes,
                },
            )
            .map_err(|_| AppError::Cryptography)?;
        content_key.zeroize();

        let mut encoded =
            Vec::with_capacity(MAGIC.len() + 6 + header_bytes.len() + ciphertext.len());
        encoded.extend_from_slice(MAGIC);
        encoded.extend_from_slice(&FORMAT_VERSION.to_be_bytes());
        encoded.extend_from_slice(&(header_bytes.len() as u32).to_be_bytes());
        encoded.extend_from_slice(&header_bytes);
        encoded.extend_from_slice(&ciphertext);
        Ok((encoded, header))
    }

    fn decode(
        container: &[u8],
        recovery_passphrase: &[u8],
    ) -> Result<(Vec<u8>, BackupHeader), AppError> {
        if container.len() < 14 || &container[..8] != MAGIC {
            return Err(AppError::BackupIntegrity);
        }
        let version = u16::from_be_bytes([container[8], container[9]]);
        if version != FORMAT_VERSION {
            return Err(AppError::BackupIntegrity);
        }
        let header_len =
            u32::from_be_bytes([container[10], container[11], container[12], container[13]])
                as usize;
        if header_len == 0 || header_len > MAX_HEADER_BYTES || 14 + header_len >= container.len() {
            return Err(AppError::BackupIntegrity);
        }
        let header_bytes = &container[14..14 + header_len];
        let header: BackupHeader =
            serde_json::from_slice(header_bytes).map_err(|_| AppError::BackupIntegrity)?;
        if header.format_version != FORMAT_VERSION
            || header.format != "AUTOVAXX_ENCRYPTED_SQLITE"
            || header.data_classification != "SYNTHETIC_ONLY"
            || header.kdf_memory_kib != KDF_MEMORY_KIB
            || header.kdf_iterations != KDF_ITERATIONS
            || header.kdf_parallelism != KDF_PARALLELISM
        {
            return Err(AppError::BackupIntegrity);
        }
        let salt = decode_fixed::<16>(&header.salt_b64)?;
        let wrap_nonce = decode_fixed::<12>(&header.wrapped_key_nonce_b64)?;
        let payload_nonce = decode_fixed::<12>(&header.payload_nonce_b64)?;
        let wrapped_key = STANDARD_NO_PAD
            .decode(&header.wrapped_content_key_b64)
            .map_err(|_| AppError::BackupIntegrity)?;
        let wrapping_key = Self::derive_wrapping_key(recovery_passphrase, &salt)?;
        let wrap_cipher =
            Aes256Gcm::new_from_slice(wrapping_key.as_ref()).map_err(|_| AppError::Cryptography)?;
        let mut wrap_aad = Vec::from(MAGIC.as_slice());
        wrap_aad.extend_from_slice(&FORMAT_VERSION.to_be_bytes());
        wrap_aad.extend_from_slice(&salt);
        let mut content_key = Zeroizing::new(
            wrap_cipher
                .decrypt(
                    Nonce::from_slice(&wrap_nonce),
                    Payload {
                        msg: &wrapped_key,
                        aad: &wrap_aad,
                    },
                )
                .map_err(|_| AppError::BackupIntegrity)?,
        );
        if content_key.len() != 32 {
            return Err(AppError::BackupIntegrity);
        }
        let content_cipher =
            Aes256Gcm::new_from_slice(&content_key).map_err(|_| AppError::Cryptography)?;
        let plaintext = content_cipher
            .decrypt(
                Nonce::from_slice(&payload_nonce),
                Payload {
                    msg: &container[14 + header_len..],
                    aad: header_bytes,
                },
            )
            .map_err(|_| AppError::BackupIntegrity)?;
        content_key.zeroize();
        if hex_sha256(&plaintext) != header.plaintext_sha256 {
            return Err(AppError::BackupIntegrity);
        }
        Ok((plaintext, header))
    }
}

impl BackupService for EncryptedBackupService {
    fn create_encrypted_backup(
        &self,
        database_path: &Path,
        destination: &Path,
        recovery_passphrase: &[u8],
    ) -> Result<BackupReceipt, AppError> {
        let parent = destination.parent().ok_or(AppError::Validation)?;
        let snapshot = Self::sqlite_snapshot(database_path, parent)?;
        let metadata = snapshot.as_file().metadata()?;
        if metadata.len() > MAX_BACKUP_BYTES {
            return Err(AppError::Validation);
        }
        let database_bytes = fs::read(snapshot.path())?;
        let (encoded, header) = Self::encode(&database_bytes, recovery_passphrase)?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)?;
        output.write_all(&encoded)?;
        output.sync_all()?;
        Ok(BackupReceipt {
            format_version: FORMAT_VERSION,
            plaintext_sha256: header.plaintext_sha256,
            encrypted_size_bytes: encoded.len() as u64,
        })
    }

    fn stage_restore(
        &self,
        backup_path: &Path,
        staging_directory: &Path,
        recovery_passphrase: &[u8],
    ) -> Result<StagedRestore, AppError> {
        let metadata = fs::metadata(backup_path)?;
        if metadata.len() > MAX_BACKUP_BYTES {
            return Err(AppError::Validation);
        }
        let container = fs::read(backup_path)?;
        let (plaintext, header) = Self::decode(&container, recovery_passphrase)?;
        fs::create_dir_all(staging_directory)?;
        let staged_database_path =
            staging_directory.join(format!("autovaxx-restore-{}.sqlite", Uuid::new_v4()));
        let mut staged = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staged_database_path)?;
        staged.write_all(&plaintext)?;
        staged.sync_all()?;
        drop(staged);

        let connection =
            Connection::open_with_flags(&staged_database_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(|_| AppError::BackupIntegrity)?;
        let check: String = connection
            .query_row("PRAGMA quick_check", [], |row| row.get(0))
            .map_err(|_| AppError::BackupIntegrity)?;
        let classification: String = connection
            .query_row(
                "SELECT value FROM app_metadata WHERE key = 'data_classification'",
                [],
                |row| row.get(0),
            )
            .map_err(|_| AppError::BackupIntegrity)?;
        if check != "ok" || classification != "SYNTHETIC_ONLY" {
            let _ = fs::remove_file(&staged_database_path);
            return Err(AppError::BackupIntegrity);
        }
        Ok(StagedRestore {
            staged_database_path,
            plaintext_sha256: header.plaintext_sha256,
        })
    }

    fn cutover(&self, staged: &StagedRestore, destination: &Path) -> Result<(), AppError> {
        if destination.exists() || !staged.staged_database_path.exists() {
            return Err(AppError::Validation);
        }
        fs::rename(&staged.staged_database_path, destination)?;
        Ok(())
    }
}

fn decode_fixed<const N: usize>(value: &str) -> Result<[u8; N], AppError> {
    let bytes = STANDARD_NO_PAD
        .decode(value)
        .map_err(|_| AppError::BackupIntegrity)?;
    bytes.try_into().map_err(|_| AppError::BackupIntegrity)
}

fn hex_sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_database(path: &Path) {
        let connection = Connection::open(path).unwrap();
        connection.execute_batch(
            "CREATE TABLE app_metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO app_metadata (key, value) VALUES ('data_classification', 'SYNTHETIC_ONLY');
             CREATE TABLE patients (patient_id TEXT PRIMARY KEY, fictional_name TEXT NOT NULL);
             INSERT INTO patients VALUES ('00000000-0000-4000-8000-000000000001', 'Synthetic Test Patient');",
        ).unwrap();
    }

    #[test]
    fn encrypted_backup_stages_and_cuts_over() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.sqlite");
        let backup = temp.path().join("manual.avxbak");
        let restored = temp.path().join("restored.sqlite");
        synthetic_database(&source);
        let service = EncryptedBackupService;
        let receipt = service
            .create_encrypted_backup(&source, &backup, b"synthetic-recovery-passphrase")
            .unwrap();
        assert_eq!(receipt.format_version, 1);
        assert_ne!(fs::read(&source).unwrap(), fs::read(&backup).unwrap());
        let staged = service
            .stage_restore(&backup, temp.path(), b"synthetic-recovery-passphrase")
            .unwrap();
        service.cutover(&staged, &restored).unwrap();
        let connection = Connection::open(restored).unwrap();
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM patients", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn corrupted_backup_is_rejected_without_staged_database() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.sqlite");
        let backup = temp.path().join("manual.avxbak");
        synthetic_database(&source);
        let service = EncryptedBackupService;
        service
            .create_encrypted_backup(&source, &backup, b"synthetic-recovery-passphrase")
            .unwrap();
        let mut bytes = fs::read(&backup).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0x01;
        fs::write(&backup, bytes).unwrap();
        assert!(matches!(
            service.stage_restore(&backup, temp.path(), b"synthetic-recovery-passphrase"),
            Err(AppError::BackupIntegrity)
        ));
    }

    #[test]
    fn cutover_never_overwrites_an_existing_database() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.sqlite");
        let backup = temp.path().join("manual.avxbak");
        synthetic_database(&source);
        let service = EncryptedBackupService;
        service
            .create_encrypted_backup(&source, &backup, b"synthetic-recovery-passphrase")
            .unwrap();
        let staged = service
            .stage_restore(&backup, temp.path(), b"synthetic-recovery-passphrase")
            .unwrap();
        assert!(matches!(
            service.cutover(&staged, &source),
            Err(AppError::Validation)
        ));
    }
}
