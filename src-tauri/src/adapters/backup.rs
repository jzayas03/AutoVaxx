use std::fs;
use std::io::Write;
use std::path::Path;

use aes_gcm::aead::{Aead, Payload};
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::Engine;
use base64::engine::general_purpose::STANDARD_NO_PAD;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

#[cfg(feature = "sqlcipher")]
use crate::adapters::{Database, DatabaseKeyLifecycle};
use crate::error::AppError;
use crate::ports::{
    BackupReceipt, BackupService, EncryptedSnapshotSource, RestoreSummary, SecretStore,
    StagedRestore,
};

const MAGIC: &[u8; 8] = b"AVXBAK02";
const FORMAT_VERSION: u16 = 2;
#[cfg(feature = "sqlcipher")]
const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_BACKUP_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_PAYLOAD_METADATA_BYTES: usize = 64 * 1024;
const KDF_MEMORY_KIB: u32 = 65_536;
const KDF_ITERATIONS: u32 = 3;
const KDF_PARALLELISM: u32 = 1;
const RECOVERY_WRAP_ALGORITHM: &str = "ARGON2ID-AES-256-GCM";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BackupHeader {
    format: String,
    format_version: u16,
    software_version: String,
    schema_version: u32,
    backup_id: Uuid,
    created_at_utc: String,
    payload_cipher: String,
    database_cipher: String,
    key_wrap: String,
    kdf: String,
    kdf_memory_kib: u32,
    kdf_iterations: u32,
    kdf_parallelism: u32,
    salt_b64: String,
    wrapped_key_nonce_b64: String,
    wrapped_content_key_b64: String,
    payload_nonce_b64: String,
    compatibility: String,
    data_classification: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EncryptedPayloadMetadata {
    schema_version: u32,
    snapshot_sha256: String,
    snapshot_size_bytes: u64,
    audit_chain_included: bool,
    content_package_references: Vec<String>,
}

#[cfg(feature = "sqlcipher")]
struct DecodedBackup {
    header: BackupHeader,
    payload: Zeroizing<Vec<u8>>,
}

#[cfg(feature = "sqlcipher")]
struct UnpackedBackup {
    header: BackupHeader,
    metadata: EncryptedPayloadMetadata,
    snapshot_key: Zeroizing<Vec<u8>>,
    encrypted_snapshot: Vec<u8>,
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
        recovery_secret: &[u8],
        salt: &[u8],
    ) -> Result<Zeroizing<[u8; 32]>, AppError> {
        if recovery_secret.len() < 16 {
            return Err(AppError::Validation);
        }
        let params = Params::new(KDF_MEMORY_KIB, KDF_ITERATIONS, KDF_PARALLELISM, Some(32))
            .map_err(|_| AppError::Cryptography)?;
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        let mut key = Zeroizing::new([0_u8; 32]);
        argon2
            .hash_password_into(recovery_secret, salt, key.as_mut())
            .map_err(|_| AppError::Cryptography)?;
        Ok(key)
    }

    fn encode(
        encrypted_snapshot: &[u8],
        snapshot_key: &[u8],
        summary: &RestoreSummary,
        recovery_secret: &[u8],
    ) -> Result<(Vec<u8>, BackupHeader, String), AppError> {
        if snapshot_key.len() != 32 {
            return Err(AppError::SecretCorrupted);
        }
        let backup_id = Uuid::new_v4();
        let salt = Self::random::<16>()?;
        let wrap_nonce = Self::random::<12>()?;
        let payload_nonce = Self::random::<12>()?;
        let mut content_key = Zeroizing::new(Self::random::<32>()?);
        let wrapping_key = Self::derive_wrapping_key(recovery_secret, &salt)?;
        let wrap_cipher =
            Aes256Gcm::new_from_slice(wrapping_key.as_ref()).map_err(|_| AppError::Cryptography)?;
        let wrap_aad = wrapping_aad(backup_id, &salt);
        let wrapped_content_key = wrap_cipher
            .encrypt(
                Nonce::from_slice(&wrap_nonce),
                Payload {
                    msg: content_key.as_ref(),
                    aad: &wrap_aad,
                },
            )
            .map_err(|_| AppError::Cryptography)?;

        let snapshot_sha256 = hex_sha256(encrypted_snapshot);
        let payload_metadata = EncryptedPayloadMetadata {
            schema_version: summary.schema_version,
            snapshot_sha256: snapshot_sha256.clone(),
            snapshot_size_bytes: encrypted_snapshot.len() as u64,
            audit_chain_included: true,
            content_package_references: Vec::new(),
        };
        let payload_metadata_bytes = serde_json::to_vec(&payload_metadata)?;
        if payload_metadata_bytes.len() > MAX_PAYLOAD_METADATA_BYTES {
            return Err(AppError::Validation);
        }
        let mut payload_plaintext = Zeroizing::new(Vec::with_capacity(
            4 + payload_metadata_bytes.len() + snapshot_key.len() + encrypted_snapshot.len(),
        ));
        payload_plaintext.extend_from_slice(&(payload_metadata_bytes.len() as u32).to_be_bytes());
        payload_plaintext.extend_from_slice(&payload_metadata_bytes);
        payload_plaintext.extend_from_slice(snapshot_key);
        payload_plaintext.extend_from_slice(encrypted_snapshot);

        let header = BackupHeader {
            format: "AUTOVAXX_ENCRYPTED_SQLCIPHER_SNAPSHOT".to_owned(),
            format_version: FORMAT_VERSION,
            software_version: env!("CARGO_PKG_VERSION").to_owned(),
            schema_version: summary.schema_version,
            backup_id,
            created_at_utc: Utc::now().to_rfc3339(),
            payload_cipher: "AES-256-GCM".to_owned(),
            database_cipher: "SQLCIPHER".to_owned(),
            key_wrap: RECOVERY_WRAP_ALGORITHM.to_owned(),
            kdf: "ARGON2ID-1.3".to_owned(),
            kdf_memory_kib: KDF_MEMORY_KIB,
            kdf_iterations: KDF_ITERATIONS,
            kdf_parallelism: KDF_PARALLELISM,
            salt_b64: STANDARD_NO_PAD.encode(salt),
            wrapped_key_nonce_b64: STANDARD_NO_PAD.encode(wrap_nonce),
            wrapped_content_key_b64: STANDARD_NO_PAD.encode(wrapped_content_key),
            payload_nonce_b64: STANDARD_NO_PAD.encode(payload_nonce),
            compatibility: "PLATFORM_NEUTRAL_V1".to_owned(),
            data_classification: "SYNTHETIC_ONLY".to_owned(),
        };
        let header_bytes = serde_json::to_vec(&header)?;
        let content_cipher =
            Aes256Gcm::new_from_slice(content_key.as_ref()).map_err(|_| AppError::Cryptography)?;
        let ciphertext = content_cipher
            .encrypt(
                Nonce::from_slice(&payload_nonce),
                Payload {
                    msg: payload_plaintext.as_ref(),
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
        Ok((encoded, header, snapshot_sha256))
    }

    #[cfg(feature = "sqlcipher")]
    fn decode(container: &[u8], recovery_secret: &[u8]) -> Result<DecodedBackup, AppError> {
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
        validate_header(&header)?;
        let salt = decode_fixed::<16>(&header.salt_b64)?;
        let wrap_nonce = decode_fixed::<12>(&header.wrapped_key_nonce_b64)?;
        let payload_nonce = decode_fixed::<12>(&header.payload_nonce_b64)?;
        let wrapped_key = STANDARD_NO_PAD
            .decode(&header.wrapped_content_key_b64)
            .map_err(|_| AppError::BackupIntegrity)?;
        let wrapping_key = Self::derive_wrapping_key(recovery_secret, &salt)?;
        let wrap_cipher =
            Aes256Gcm::new_from_slice(wrapping_key.as_ref()).map_err(|_| AppError::Cryptography)?;
        let mut content_key = Zeroizing::new(
            wrap_cipher
                .decrypt(
                    Nonce::from_slice(&wrap_nonce),
                    Payload {
                        msg: &wrapped_key,
                        aad: &wrapping_aad(header.backup_id, &salt),
                    },
                )
                .map_err(|_| AppError::BackupIntegrity)?,
        );
        if content_key.len() != 32 {
            return Err(AppError::BackupIntegrity);
        }
        let content_cipher =
            Aes256Gcm::new_from_slice(&content_key).map_err(|_| AppError::Cryptography)?;
        let payload = Zeroizing::new(
            content_cipher
                .decrypt(
                    Nonce::from_slice(&payload_nonce),
                    Payload {
                        msg: &container[14 + header_len..],
                        aad: header_bytes,
                    },
                )
                .map_err(|_| AppError::BackupIntegrity)?,
        );
        content_key.zeroize();
        Ok(DecodedBackup { header, payload })
    }

    #[cfg(feature = "sqlcipher")]
    fn unpack_payload(decoded: DecodedBackup) -> Result<UnpackedBackup, AppError> {
        if decoded.payload.len() < 4 + 32 {
            return Err(AppError::BackupIntegrity);
        }
        let metadata_len = u32::from_be_bytes([
            decoded.payload[0],
            decoded.payload[1],
            decoded.payload[2],
            decoded.payload[3],
        ]) as usize;
        if metadata_len == 0
            || metadata_len > MAX_PAYLOAD_METADATA_BYTES
            || 4 + metadata_len + 32 >= decoded.payload.len()
        {
            return Err(AppError::BackupIntegrity);
        }
        let metadata: EncryptedPayloadMetadata =
            serde_json::from_slice(&decoded.payload[4..4 + metadata_len])
                .map_err(|_| AppError::BackupIntegrity)?;
        let key_start = 4 + metadata_len;
        let snapshot_key = Zeroizing::new(decoded.payload[key_start..key_start + 32].to_vec());
        let snapshot = decoded.payload[key_start + 32..].to_vec();
        if metadata.schema_version != decoded.header.schema_version
            || metadata.snapshot_size_bytes != snapshot.len() as u64
            || metadata.snapshot_sha256 != hex_sha256(&snapshot)
            || !metadata.audit_chain_included
        {
            return Err(AppError::BackupIntegrity);
        }
        Ok(UnpackedBackup {
            header: decoded.header,
            metadata,
            snapshot_key,
            encrypted_snapshot: snapshot,
        })
    }
}

impl BackupService for EncryptedBackupService {
    fn create_encrypted_backup(
        &self,
        source: &dyn EncryptedSnapshotSource,
        destination: &Path,
        recovery_secret: &[u8],
    ) -> Result<BackupReceipt, AppError> {
        let parent = destination.parent().ok_or(AppError::Validation)?;
        fs::create_dir_all(parent)?;
        if destination.exists() {
            return Err(AppError::Validation);
        }

        let snapshot_directory = tempfile::tempdir_in(parent)?;
        let snapshot_path = snapshot_directory.path().join("snapshot.sqlcipher");
        let mut snapshot_key = Zeroizing::new(Self::random::<32>()?);
        let summary = source.write_encrypted_snapshot(&snapshot_path, snapshot_key.as_ref())?;
        let metadata = fs::metadata(&snapshot_path)?;
        if metadata.len() == 0 || metadata.len() > MAX_BACKUP_BYTES {
            return Err(AppError::Validation);
        }
        let encrypted_snapshot = fs::read(&snapshot_path)?;
        let (encoded, header, snapshot_sha256) = Self::encode(
            &encrypted_snapshot,
            snapshot_key.as_ref(),
            &summary,
            recovery_secret,
        )?;
        snapshot_key.zeroize();

        let mut output = tempfile::NamedTempFile::new_in(parent)?;
        output.write_all(&encoded)?;
        output.as_file().sync_all()?;
        output
            .persist_noclobber(destination)
            .map_err(|error| AppError::Io(error.error))?;
        Ok(BackupReceipt {
            backup_id: header.backup_id,
            format_version: FORMAT_VERSION,
            snapshot_sha256,
            encrypted_size_bytes: encoded.len() as u64,
        })
    }

    fn stage_restore(
        &self,
        backup_path: &Path,
        staging_directory: &Path,
        recovery_secret: &[u8],
    ) -> Result<StagedRestore, AppError> {
        #[cfg(not(feature = "sqlcipher"))]
        {
            let _ = (backup_path, staging_directory, recovery_secret);
            Err(AppError::Configuration)
        }
        #[cfg(feature = "sqlcipher")]
        {
            let metadata = fs::metadata(backup_path)?;
            if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_BACKUP_BYTES {
                return Err(AppError::Validation);
            }
            let container = fs::read(backup_path)?;
            let unpacked = Self::unpack_payload(Self::decode(&container, recovery_secret)?)?;
            fs::create_dir_all(staging_directory)?;
            let mut staged = tempfile::NamedTempFile::new_in(staging_directory)?;
            staged.write_all(&unpacked.encrypted_snapshot)?;
            staged.as_file().sync_all()?;

            let summary = Database::validate_encrypted_file(staged.path(), &unpacked.snapshot_key)?;

            if summary.schema_version != unpacked.metadata.schema_version {
                return Err(AppError::BackupIntegrity);
            }
            let (_, staged_database_path) =
                staged.keep().map_err(|error| AppError::Io(error.error))?;
            Ok(StagedRestore {
                backup_id: unpacked.header.backup_id,
                staged_database_path,
                snapshot_sha256: unpacked.metadata.snapshot_sha256,
                summary,
                snapshot_key: unpacked.snapshot_key,
            })
        }
    }

    fn cutover(
        &self,
        staged: StagedRestore,
        destination: &Path,
        secret_store: &dyn SecretStore,
    ) -> Result<(), AppError> {
        #[cfg(feature = "sqlcipher")]
        {
            Database::validate_encrypted_file(&staged.staged_database_path, &staged.snapshot_key)?;
            DatabaseKeyLifecycle::new(secret_store).adopt_encrypted_database(
                &staged.staged_database_path,
                destination,
                &staged.snapshot_key,
            )?;
            Ok(())
        }
        #[cfg(not(feature = "sqlcipher"))]
        {
            let _ = (staged, destination, secret_store);
            Err(AppError::Configuration)
        }
    }
}

#[cfg(feature = "sqlcipher")]
fn validate_header(header: &BackupHeader) -> Result<(), AppError> {
    if header.format_version != FORMAT_VERSION
        || header.format != "AUTOVAXX_ENCRYPTED_SQLCIPHER_SNAPSHOT"
        || header.schema_version != crate::adapters::SCHEMA_VERSION
        || header.payload_cipher != "AES-256-GCM"
        || header.database_cipher != "SQLCIPHER"
        || header.key_wrap != RECOVERY_WRAP_ALGORITHM
        || header.kdf != "ARGON2ID-1.3"
        || header.kdf_memory_kib != KDF_MEMORY_KIB
        || header.kdf_iterations != KDF_ITERATIONS
        || header.kdf_parallelism != KDF_PARALLELISM
        || header.compatibility != "PLATFORM_NEUTRAL_V1"
        || header.data_classification != "SYNTHETIC_ONLY"
        || header.software_version.is_empty()
    {
        return Err(AppError::BackupIntegrity);
    }
    chrono::DateTime::parse_from_rfc3339(&header.created_at_utc)
        .map_err(|_| AppError::BackupIntegrity)?;
    Ok(())
}

fn wrapping_aad(backup_id: Uuid, salt: &[u8]) -> Vec<u8> {
    let mut aad = Vec::from(MAGIC.as_slice());
    aad.extend_from_slice(&FORMAT_VERSION.to_be_bytes());
    aad.extend_from_slice(backup_id.as_bytes());
    aad.extend_from_slice(salt);
    aad
}

#[cfg(feature = "sqlcipher")]
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

#[cfg(all(test, feature = "sqlcipher"))]
mod tests {
    use super::*;
    use crate::adapters::FakeSecretStore;

    const RECOVERY_SECRET: &[u8] = b"synthetic-recovery-passphrase";
    const SENTINEL: &[u8] = b"SYNTHETIC-PHI-EQUIVALENT-SENTINEL";

    fn encrypted_source<'a>(
        temp: &tempfile::TempDir,
        store: &'a FakeSecretStore,
    ) -> (Database, DatabaseKeyLifecycle<'a>) {
        let path = temp.path().join("source.sqlite");
        let lifecycle = DatabaseKeyLifecycle::new(store);
        let database = lifecycle.create_encrypted_database(&path).unwrap();
        database
            .execute_test_sql(
                "CREATE TABLE synthetic_markers (id INTEGER PRIMARY KEY, marker TEXT NOT NULL);
                 INSERT INTO synthetic_markers (marker) VALUES ('SYNTHETIC-PHI-EQUIVALENT-SENTINEL');",
            )
            .unwrap();
        (database, lifecycle)
    }

    #[test]
    fn backup_and_restore_never_stage_a_plaintext_sqlite_database() {
        let temp = tempfile::tempdir().unwrap();
        let store = FakeSecretStore::new();
        let (database, lifecycle) = encrypted_source(&temp, &store);
        let backup = temp.path().join("manual.avxbak");
        let restored = temp.path().join("restored.sqlite");
        let service = EncryptedBackupService;
        let receipt = service
            .create_encrypted_backup(&database, &backup, RECOVERY_SECRET)
            .unwrap();
        assert_eq!(receipt.format_version, FORMAT_VERSION);
        assert!(!contains_marker(&fs::read(&backup).unwrap(), SENTINEL));

        let staged = service
            .stage_restore(&backup, temp.path(), RECOVERY_SECRET)
            .unwrap();
        let staged_bytes = fs::read(&staged.staged_database_path).unwrap();
        assert!(!staged_bytes.starts_with(b"SQLite format 3\0"));
        assert!(!contains_marker(&staged_bytes, SENTINEL));
        service.cutover(staged, &restored, &store).unwrap();
        drop(database);
        assert!(lifecycle.open_encrypted_database(&restored).is_ok());
    }

    #[test]
    fn every_backup_has_independent_keys_and_authenticated_header() {
        let temp = tempfile::tempdir().unwrap();
        let store = FakeSecretStore::new();
        let (database, _) = encrypted_source(&temp, &store);
        let first = temp.path().join("first.avxbak");
        let second = temp.path().join("second.avxbak");
        let service = EncryptedBackupService;
        let first_receipt = service
            .create_encrypted_backup(&database, &first, RECOVERY_SECRET)
            .unwrap();
        let second_receipt = service
            .create_encrypted_backup(&database, &second, RECOVERY_SECRET)
            .unwrap();
        assert_ne!(first_receipt.backup_id, second_receipt.backup_id);
        assert_ne!(fs::read(&first).unwrap(), fs::read(&second).unwrap());

        let mut tampered = fs::read(&first).unwrap();
        let header_start = 14;
        tampered[header_start + 20] ^= 1;
        fs::write(&first, tampered).unwrap();
        assert!(matches!(
            service.stage_restore(&first, temp.path(), RECOVERY_SECRET),
            Err(AppError::BackupIntegrity)
        ));
    }

    #[test]
    fn wrong_secret_ciphertext_tag_truncation_and_future_version_fail_cleanly() {
        let temp = tempfile::tempdir().unwrap();
        let store = FakeSecretStore::new();
        let (database, _) = encrypted_source(&temp, &store);
        let backup = temp.path().join("manual.avxbak");
        let service = EncryptedBackupService;
        service
            .create_encrypted_backup(&database, &backup, RECOVERY_SECRET)
            .unwrap();
        let original = fs::read(&backup).unwrap();

        assert!(matches!(
            service.stage_restore(&backup, temp.path(), b"wrong-recovery-secret"),
            Err(AppError::BackupIntegrity)
        ));
        for mutation in [
            "ciphertext",
            "tag",
            "truncated",
            "future-version",
            "prototype-v1",
        ] {
            let candidate = temp.path().join(format!("{mutation}.avxbak"));
            let mut bytes = original.clone();
            match mutation {
                "ciphertext" => {
                    let index = bytes.len() - 17;
                    bytes[index] ^= 1;
                }
                "tag" => {
                    let index = bytes.len() - 1;
                    bytes[index] ^= 1;
                }
                "truncated" => bytes.truncate(bytes.len() - 12),
                "future-version" => bytes[8..10].copy_from_slice(&99_u16.to_be_bytes()),
                "prototype-v1" => {
                    bytes[..8].copy_from_slice(b"AVXBAK01");
                    bytes[8..10].copy_from_slice(&1_u16.to_be_bytes());
                }
                _ => unreachable!(),
            }
            fs::write(&candidate, bytes).unwrap();
            assert!(matches!(
                service.stage_restore(&candidate, temp.path(), RECOVERY_SECRET),
                Err(AppError::BackupIntegrity)
            ));
        }

        let header_len =
            u32::from_be_bytes([original[10], original[11], original[12], original[13]]) as usize;
        let mut header: serde_json::Value =
            serde_json::from_slice(&original[14..14 + header_len]).unwrap();
        header.as_object_mut().unwrap().remove("softwareVersion");
        let incomplete_header = serde_json::to_vec(&header).unwrap();
        let mut missing_metadata = Vec::new();
        missing_metadata.extend_from_slice(MAGIC);
        missing_metadata.extend_from_slice(&FORMAT_VERSION.to_be_bytes());
        missing_metadata.extend_from_slice(&(incomplete_header.len() as u32).to_be_bytes());
        missing_metadata.extend_from_slice(&incomplete_header);
        missing_metadata.extend_from_slice(&original[14 + header_len..]);
        let candidate = temp.path().join("missing-header-metadata.avxbak");
        fs::write(&candidate, missing_metadata).unwrap();
        assert!(matches!(
            service.stage_restore(&candidate, temp.path(), RECOVERY_SECRET),
            Err(AppError::BackupIntegrity)
        ));
    }

    #[test]
    fn schema_and_audit_tampering_inside_valid_envelope_are_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let store = FakeSecretStore::new();
        let (database, lifecycle) = encrypted_source(&temp, &store);
        let recovered = lifecycle.recover(database.path()).unwrap();
        database
            .execute_test_sql("UPDATE app_metadata SET value = '999' WHERE key = 'schema_version'")
            .unwrap();
        drop(database);
        let snapshot = fs::read(temp.path().join("source.sqlite")).unwrap();
        let summary = RestoreSummary {
            schema_version: crate::adapters::SCHEMA_VERSION,
            audit_event_count: 0,
            patient_count: 0,
            encounter_count: 0,
        };
        let (container, _, _) =
            EncryptedBackupService::encode(&snapshot, &recovered.key, &summary, RECOVERY_SECRET)
                .unwrap();
        let backup = temp.path().join("schema-tampered.avxbak");
        fs::write(&backup, container).unwrap();
        assert!(matches!(
            EncryptedBackupService.stage_restore(&backup, temp.path(), RECOVERY_SECRET),
            Err(AppError::BackupIntegrity)
        ));

        let reopened = lifecycle
            .open_encrypted_database(&temp.path().join("source.sqlite"))
            .unwrap();
        reopened
            .execute_test_sql(
                "UPDATE app_metadata SET value = '1' WHERE key = 'schema_version';
                 INSERT INTO audit_events (
                    audit_event_id, occurred_at_utc, recorded_at_utc, actor_id, session_id,
                    workstation_id, facility_id, action_code, entity_type, entity_id,
                    entity_revision, outcome, correlation_id, software_version, schema_version,
                    metadata_json, previous_hash, event_hash
                 ) VALUES (
                    '00000000-0000-4000-8000-000000000001',
                    '2026-01-01T00:00:00+00:00', '2026-01-01T00:00:00+00:00', NULL, NULL,
                    '00000000-0000-4000-8000-000000000002',
                    '00000000-0000-4000-8000-000000000003', 'SYNTHETIC_TAMPER',
                    'TEST', 'SYNTHETIC', NULL, 'SUCCEEDED',
                    '00000000-0000-4000-8000-000000000004', '0.1.0', 1, '{}', NULL,
                    'deliberately-invalid-audit-hash'
                 );",
            )
            .unwrap();
        drop(reopened);
        let snapshot = fs::read(temp.path().join("source.sqlite")).unwrap();
        let (container, _, _) =
            EncryptedBackupService::encode(&snapshot, &recovered.key, &summary, RECOVERY_SECRET)
                .unwrap();
        let backup = temp.path().join("audit-tampered.avxbak");
        fs::write(&backup, container).unwrap();
        assert!(matches!(
            EncryptedBackupService.stage_restore(&backup, temp.path(), RECOVERY_SECRET),
            Err(AppError::BackupIntegrity)
        ));
    }

    #[test]
    fn failed_cutover_preserves_active_database_and_cleans_staging() {
        let temp = tempfile::tempdir().unwrap();
        let store = FakeSecretStore::new();
        let (database, lifecycle) = encrypted_source(&temp, &store);
        let backup = temp.path().join("manual.avxbak");
        let service = EncryptedBackupService;
        service
            .create_encrypted_backup(&database, &backup, RECOVERY_SECRET)
            .unwrap();
        let staged = service
            .stage_restore(&backup, temp.path(), RECOVERY_SECRET)
            .unwrap();
        let staged_path = staged.staged_database_path.clone();
        let active = database.path().to_path_buf();
        assert!(matches!(
            service.cutover(staged, &active, &store),
            Err(AppError::Validation)
        ));
        assert!(!staged_path.exists());
        assert!(lifecycle.open_encrypted_database(&active).is_ok());
    }

    #[test]
    fn abandoned_restore_stage_is_removed_without_touching_active_database() {
        eprintln!("backup-test: before fixture setup");
        let temp = tempfile::tempdir().unwrap();
        let store = FakeSecretStore::new();
        let (database, lifecycle) = encrypted_source(&temp, &store);
        eprintln!("backup-test: after fixture setup");
        let backup = temp.path().join("manual.avxbak");
        let service = EncryptedBackupService;
        service
            .create_encrypted_backup(&database, &backup, RECOVERY_SECRET)
            .unwrap();
        eprintln!("backup-test: after backup creation");
        let staged = service
            .stage_restore(&backup, temp.path(), RECOVERY_SECRET)
            .unwrap();
        eprintln!("backup-test: after restore staging");
        let staged_path = staged.staged_database_path.clone();
        assert!(staged_path.exists());
        drop(staged);
        assert!(!staged_path.exists());
        assert!(lifecycle.open_encrypted_database(database.path()).is_ok());
        eprintln!("backup-test: after active database reopen");
    }

    fn contains_marker(bytes: &[u8], marker: &[u8]) -> bool {
        bytes.windows(marker.len()).any(|window| window == marker)
    }
}
