mod migrations;
#[cfg(all(test, feature = "sqlcipher"))]
mod sqlcipher_spike;

use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Mutex;

use chrono::{DateTime, Duration, Utc};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use sha2::{Digest, Sha256};
use uuid::Uuid;
#[cfg(feature = "sqlcipher")]
use zeroize::{Zeroize, Zeroizing};

use crate::domain::{
    EncounterId, EncounterState, ExternalIdentifier, Facility, FacilityId, ImmunizationEncounter,
    Patient, PatientAddress, PatientId, PatientName, Permission, Role, SessionContext, SessionId,
    User, UserId, WorkstationId, permissions_for_role,
};
use crate::error::AppError;
use crate::ports::{
    AuditRepository, EncounterRepository, EncryptedSnapshotSource, PatientRepository,
    RestoreSummary,
};

use self::migrations::MIGRATION_001;
pub use self::migrations::SCHEMA_VERSION;

pub struct Database {
    connection: Mutex<Connection>,
    path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct AuditDraft<'a> {
    pub action: &'a str,
    pub entity_type: &'a str,
    pub entity_id: &'a str,
    pub entity_revision: Option<u64>,
    pub outcome: &'a str,
    pub correlation_id: Uuid,
    pub metadata_json: &'a str,
}

struct UnauthenticatedAuditDraft<'a> {
    actor_id: Option<UserId>,
    workstation_id: WorkstationId,
    facility_id: FacilityId,
    action: &'a str,
    outcome: &'a str,
    metadata_json: &'a str,
    now: DateTime<Utc>,
}

#[derive(Debug)]
pub struct StoredCredential {
    pub user: User,
    pub facility_id: FacilityId,
    pub password_verifier: String,
    pub failed_attempt_count: u32,
    pub failed_attempt_window_started_at: Option<DateTime<Utc>>,
    pub locked_until: Option<DateTime<Utc>>,
}

impl Database {
    pub fn open_synthetic(path: impl AsRef<Path>) -> Result<Self, AppError> {
        let path = path.as_ref().to_path_buf();
        let existing_nonempty = path
            .metadata()
            .map(|metadata| metadata.len() > 0)
            .unwrap_or(false);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(&path)?;
        Self::configure(&connection, true)?;
        if existing_nonempty {
            let has_metadata_table: bool = connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'app_metadata')",
                [],
                |row| row.get(0),
            )?;
            if !has_metadata_table {
                return Err(AppError::Configuration);
            }
            Self::assert_classification(&connection, "SYNTHETIC_ONLY")?;
        }
        Self::migrate(&connection)?;
        Self::assert_classification(&connection, "SYNTHETIC_ONLY")?;
        Ok(Self {
            connection: Mutex::new(connection),
            path,
        })
    }

    pub fn open_synthetic_in_memory() -> Result<Self, AppError> {
        let connection = Connection::open_in_memory()?;
        Self::configure(&connection, false)?;
        Self::migrate(&connection)?;
        Self::assert_classification(&connection, "SYNTHETIC_ONLY")?;
        Ok(Self {
            connection: Mutex::new(connection),
            path: PathBuf::from(":memory:"),
        })
    }

    #[cfg(feature = "sqlcipher")]
    pub fn create_encrypted(path: impl AsRef<Path>, key: &[u8]) -> Result<Self, AppError> {
        let path = path.as_ref().to_path_buf();
        if path.exists() {
            return Err(AppError::Validation);
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        #[cfg(test)]
        eprintln!("sqlcipher-test: before connection open");
        let connection = Connection::open(&path)?;
        #[cfg(test)]
        eprintln!("sqlcipher-test: after connection open");
        Self::apply_sqlcipher_key(&connection, key)?;
        #[cfg(test)]
        eprintln!("sqlcipher-test: after key application");
        Self::configure(&connection, true)?;
        #[cfg(test)]
        eprintln!("sqlcipher-test: after connection configuration");
        Self::migrate(&connection)?;
        #[cfg(test)]
        eprintln!("sqlcipher-test: after migration");
        Self::assert_classification(&connection, "SYNTHETIC_ONLY")?;
        #[cfg(test)]
        eprintln!("sqlcipher-test: after classification check");
        Ok(Self {
            connection: Mutex::new(connection),
            path,
        })
    }

    #[cfg(feature = "sqlcipher")]
    pub fn open_encrypted(path: impl AsRef<Path>, key: &[u8]) -> Result<Self, AppError> {
        let path = path.as_ref().to_path_buf();
        if !path
            .metadata()
            .map(|metadata| metadata.is_file() && metadata.len() > 0)
            .unwrap_or(false)
        {
            return Err(AppError::NotFound);
        }
        let connection = Connection::open(&path)?;
        Self::apply_sqlcipher_key(&connection, key)?;
        connection
            .query_row("SELECT COUNT(*) FROM sqlite_master", [], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(|_| AppError::DatabaseKeyInvalid)?;
        Self::configure(&connection, true)?;
        Self::assert_classification(&connection, "SYNTHETIC_ONLY")
            .map_err(|_| AppError::DatabaseKeyInvalid)?;
        Self::migrate(&connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
            path,
        })
    }

    #[cfg(feature = "sqlcipher")]
    fn apply_sqlcipher_key(connection: &Connection, key: &[u8]) -> Result<(), AppError> {
        if key.len() != 32 {
            return Err(AppError::SecretCorrupted);
        }
        let mut hex_key = Zeroizing::new(
            key.iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
        );
        let pragma_value = Zeroizing::new(format!("x'{}'", hex_key.as_str()));
        #[cfg(test)]
        eprintln!("sqlcipher-test: before key pragma");
        connection
            .pragma_update(None, "key", pragma_value.as_str())
            .map_err(|_| AppError::DatabaseKeyInvalid)?;
        #[cfg(test)]
        eprintln!("sqlcipher-test: after key pragma");
        hex_key.zeroize();
        connection
            .pragma_update(None, "cipher_memory_security", "ON")
            .map_err(|_| AppError::DatabaseKeyInvalid)?;
        #[cfg(test)]
        eprintln!("sqlcipher-test: after cipher memory security pragma");
        let cipher_version: String = connection
            .query_row("PRAGMA cipher_version", [], |row| row.get(0))
            .map_err(|_| AppError::DatabaseKeyInvalid)?;
        #[cfg(test)]
        eprintln!("sqlcipher-test: after cipher version query");
        if cipher_version.is_empty() {
            return Err(AppError::Configuration);
        }
        Ok(())
    }

    #[cfg(feature = "sqlcipher")]
    pub fn validate_encrypted_file(path: &Path, key: &[u8]) -> Result<RestoreSummary, AppError> {
        let connection =
            Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(|_| AppError::BackupIntegrity)?;
        Self::apply_sqlcipher_key(&connection, key).map_err(|_| AppError::BackupIntegrity)?;
        connection
            .query_row("SELECT COUNT(*) FROM sqlite_master", [], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(|_| AppError::BackupIntegrity)?;
        Self::validate_connection_integrity(&connection)
    }

    #[cfg(feature = "sqlcipher")]
    fn validate_connection_integrity(connection: &Connection) -> Result<RestoreSummary, AppError> {
        let quick_check: String = connection
            .query_row("PRAGMA quick_check", [], |row| row.get(0))
            .map_err(|_| AppError::BackupIntegrity)?;
        if quick_check != "ok" {
            return Err(AppError::BackupIntegrity);
        }

        let mut cipher_statement = connection
            .prepare("PRAGMA cipher_integrity_check")
            .map_err(|_| AppError::BackupIntegrity)?;
        let cipher_results = cipher_statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|_| AppError::BackupIntegrity)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| AppError::BackupIntegrity)?;
        if !cipher_results.is_empty()
            && !(cipher_results.len() == 1 && cipher_results[0].eq_ignore_ascii_case("ok"))
        {
            return Err(AppError::BackupIntegrity);
        }

        let mut foreign_key_statement = connection
            .prepare("PRAGMA foreign_key_check")
            .map_err(|_| AppError::BackupIntegrity)?;
        if foreign_key_statement
            .query([])
            .map_err(|_| AppError::BackupIntegrity)?
            .next()
            .map_err(|_| AppError::BackupIntegrity)?
            .is_some()
        {
            return Err(AppError::BackupIntegrity);
        }

        let schema_metadata: String = connection
            .query_row(
                "SELECT value FROM app_metadata WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .map_err(|_| AppError::BackupIntegrity)?;
        if schema_metadata != SCHEMA_VERSION.to_string() {
            return Err(AppError::BackupIntegrity);
        }
        let expected_checksum = format!("sha256:{}", sha256_text(MIGRATION_001));
        let migration_checksum: String = connection
            .query_row(
                "SELECT checksum FROM schema_migrations WHERE version = ?1",
                params![SCHEMA_VERSION],
                |row| row.get(0),
            )
            .map_err(|_| AppError::BackupIntegrity)?;
        if migration_checksum != expected_checksum {
            return Err(AppError::BackupIntegrity);
        }
        Self::assert_classification(connection, "SYNTHETIC_ONLY")
            .map_err(|_| AppError::BackupIntegrity)?;

        let audit_event_count = verify_audit_chain(connection)?;
        let patient_count = validated_revision_count(connection, "patients")?;
        let encounter_count = validated_revision_count(connection, "immunization_encounters")?;
        validated_revision_count(connection, "users")?;

        Ok(RestoreSummary {
            schema_version: SCHEMA_VERSION,
            audit_event_count,
            patient_count,
            encounter_count,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn configure(connection: &Connection, file_backed: bool) -> Result<(), AppError> {
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        connection.pragma_update(None, "trusted_schema", "OFF")?;
        connection.pragma_update(None, "secure_delete", "ON")?;
        if file_backed {
            connection.pragma_update(None, "journal_mode", "WAL")?;
            connection.pragma_update(None, "synchronous", "FULL")?;
        }
        Ok(())
    }

    fn migrate(connection: &Connection) -> Result<(), AppError> {
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                checksum TEXT NOT NULL,
                applied_at_utc TEXT NOT NULL
            );",
        )?;
        let checksum = format!("sha256:{}", sha256_text(MIGRATION_001));
        let existing_checksum = connection
            .query_row(
                "SELECT checksum FROM schema_migrations WHERE version = ?1",
                params![SCHEMA_VERSION],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if let Some(existing_checksum) = existing_checksum {
            if existing_checksum != checksum {
                return Err(AppError::Configuration);
            }
        } else {
            connection.execute_batch(MIGRATION_001)?;
            connection.execute(
                "INSERT INTO schema_migrations (version, checksum, applied_at_utc) VALUES (?1, ?2, ?3)",
                params![SCHEMA_VERSION, checksum, Utc::now().to_rfc3339()],
            )?;
        }
        connection.execute(
            "INSERT OR IGNORE INTO app_metadata (key, value) VALUES ('data_classification', 'SYNTHETIC_ONLY')",
            [],
        )?;
        connection.execute(
            "INSERT OR IGNORE INTO app_metadata (key, value) VALUES ('schema_version', ?1)",
            params![SCHEMA_VERSION.to_string()],
        )?;
        for role in Role::ALL {
            connection.execute(
                "INSERT OR IGNORE INTO roles (role_code, display_name) VALUES (?1, ?2)",
                params![role.code(), role.code().replace('_', " ")],
            )?;
        }
        for permission in Permission::ALL {
            connection.execute(
                "INSERT OR IGNORE INTO permissions (permission_code) VALUES (?1)",
                params![permission.code()],
            )?;
        }
        for role in Role::ALL {
            for permission in permissions_for_role(role) {
                connection.execute(
                    "INSERT OR IGNORE INTO role_permissions (role_code, permission_code) VALUES (?1, ?2)",
                    params![role.code(), permission.code()],
                )?;
            }
        }
        Ok(())
    }

    fn assert_classification(connection: &Connection, expected: &str) -> Result<(), AppError> {
        let actual: String = connection.query_row(
            "SELECT value FROM app_metadata WHERE key = 'data_classification'",
            [],
            |row| row.get(0),
        )?;
        if actual == expected {
            Ok(())
        } else {
            Err(AppError::Configuration)
        }
    }

    pub fn create_facility_and_workstation(
        &self,
        facility: &Facility,
        workstation_id: WorkstationId,
        workstation_label: &str,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| AppError::Persistence(rusqlite::Error::InvalidQuery))?;
        connection.execute(
            "INSERT INTO facilities (facility_id, name, timezone, active, created_at_utc) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![facility.facility_id.to_string(), facility.name, facility.timezone, facility.active, now.to_rfc3339()],
        )?;
        connection.execute(
            "INSERT INTO workstations (workstation_id, facility_id, label, active, created_at_utc) VALUES (?1, ?2, ?3, 1, ?4)",
            params![workstation_id.to_string(), facility.facility_id.to_string(), workstation_label, now.to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn create_user(
        &self,
        user: &User,
        facility_id: FacilityId,
        password_verifier: &str,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| AppError::Persistence(rusqlite::Error::InvalidQuery))?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute(
            "INSERT INTO users (user_id, facility_id, username, display_name, password_verifier, verifier_version, active, created_at_utc)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?7)",
            params![user.user_id.to_string(), facility_id.to_string(), user.username, user.display_name, password_verifier, user.active, now.to_rfc3339()],
        )?;
        for role in &user.roles {
            transaction.execute(
                "INSERT INTO user_roles (user_id, role_code, assigned_at_utc) VALUES (?1, ?2, ?3)",
                params![user.user_id.to_string(), role.code(), now.to_rfc3339()],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn credential_by_username(
        &self,
        username: &str,
    ) -> Result<Option<StoredCredential>, AppError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| AppError::Persistence(rusqlite::Error::InvalidQuery))?;
        let row = connection.query_row(
            "SELECT user_id, facility_id, username, display_name, active, password_verifier, failed_attempt_count,
                    failed_attempt_window_started_at_utc, locked_until_utc
             FROM users WHERE username = ?1",
            params![username],
            |row| {
                Ok((
                    row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?, row.get::<_, bool>(4)?, row.get::<_, String>(5)?,
                    row.get::<_, u32>(6)?, row.get::<_, Option<String>>(7)?, row.get::<_, Option<String>>(8)?,
                ))
            },
        ).optional()?;
        let Some((
            user_id,
            facility_id,
            username,
            display_name,
            active,
            password_verifier,
            failures,
            window,
            locked,
        )) = row
        else {
            return Ok(None);
        };
        let user_id = UserId(Uuid::from_str(&user_id).map_err(|_| AppError::Validation)?);
        let roles = Self::roles_for_user(&connection, user_id)?;
        Ok(Some(StoredCredential {
            user: User {
                user_id,
                username,
                display_name,
                active,
                roles,
            },
            facility_id: FacilityId(parse_uuid(&facility_id)?),
            password_verifier,
            failed_attempt_count: failures,
            failed_attempt_window_started_at: parse_optional_time(window)?,
            locked_until: parse_optional_time(locked)?,
        }))
    }

    fn roles_for_user(connection: &Connection, user_id: UserId) -> Result<Vec<Role>, AppError> {
        let mut statement = connection
            .prepare("SELECT role_code FROM user_roles WHERE user_id = ?1 ORDER BY role_code")?;
        let codes =
            statement.query_map(params![user_id.to_string()], |row| row.get::<_, String>(0))?;
        let mut roles = Vec::new();
        for code in codes {
            roles.push(Role::parse(&code?).ok_or(AppError::Validation)?);
        }
        Ok(roles)
    }

    pub fn record_failed_login(
        &self,
        user_id: UserId,
        facility_id: FacilityId,
        workstation_id: WorkstationId,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| AppError::Persistence(rusqlite::Error::InvalidQuery))?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current: (u32, Option<String>) = transaction.query_row(
            "SELECT failed_attempt_count, failed_attempt_window_started_at_utc FROM users WHERE user_id = ?1",
            params![user_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let window_start = parse_optional_time(current.1)?;
        let within_window = window_start.is_some_and(|start| now - start <= Duration::minutes(15));
        let attempts = if within_window { current.0 + 1 } else { 1 };
        let new_window = if within_window {
            window_start.unwrap()
        } else {
            now
        };
        let locked_until = (attempts >= 5).then(|| now + Duration::minutes(15));
        transaction.execute(
            "UPDATE users SET failed_attempt_count = ?1, failed_attempt_window_started_at_utc = ?2, locked_until_utc = ?3, revision = revision + 1 WHERE user_id = ?4",
            params![attempts, new_window.to_rfc3339(), locked_until.map(|value| value.to_rfc3339()), user_id.to_string()],
        )?;
        append_unauthenticated_audit(
            &transaction,
            &UnauthenticatedAuditDraft {
                actor_id: Some(user_id),
                workstation_id,
                facility_id,
                action: "AUTHENTICATION_FAILED",
                outcome: "DENIED",
                metadata_json: r#"{"errorCode":"INVALID_CREDENTIAL"}"#,
                now,
            },
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn create_session(
        &self,
        user: &User,
        workstation_id: WorkstationId,
        raw_token: &str,
        now: DateTime<Utc>,
    ) -> Result<SessionContext, AppError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| AppError::Persistence(rusqlite::Error::InvalidQuery))?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let facility_id: String = transaction.query_row(
            "SELECT facility_id FROM users WHERE user_id = ?1",
            params![user.user_id.to_string()],
            |row| row.get(0),
        )?;
        let session_id = SessionId::new();
        let expires_at = now + Duration::minutes(15);
        transaction.execute(
            "INSERT INTO auth_sessions (
                session_id, token_sha256, user_id, workstation_id, created_at_utc,
                expires_at_utc, recent_auth_at_utc, last_activity_at_utc
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?5, ?5)",
            params![
                session_id.to_string(),
                sha256_text(raw_token),
                user.user_id.to_string(),
                workstation_id.to_string(),
                now.to_rfc3339(),
                expires_at.to_rfc3339(),
            ],
        )?;
        transaction.execute(
            "UPDATE users SET failed_attempt_count = 0, failed_attempt_window_started_at_utc = NULL, locked_until_utc = NULL,
                    revision = CASE WHEN failed_attempt_count > 0 OR locked_until_utc IS NOT NULL THEN revision + 1 ELSE revision END
             WHERE user_id = ?1",
            params![user.user_id.to_string()],
        )?;
        let context = SessionContext {
            user_id: user.user_id,
            session_id,
            workstation_id,
            facility_id: FacilityId(
                Uuid::from_str(&facility_id).map_err(|_| AppError::Validation)?,
            ),
            roles: user.roles.clone(),
            authenticated_at: now,
            recent_auth_at: now,
            expires_at,
        };
        append_audit(
            &transaction,
            &context,
            &AuditDraft {
                action: "AUTHENTICATION_SUCCEEDED",
                entity_type: "USER_SESSION",
                entity_id: &session_id.to_string(),
                entity_revision: None,
                outcome: "SUCCEEDED",
                correlation_id: Uuid::new_v4(),
                metadata_json: r#"{"authenticationMethod":"LOCAL_PASSWORD"}"#,
            },
        )?;
        transaction.commit()?;
        Ok(context)
    }

    pub fn session_by_token(
        &self,
        raw_token: &str,
        now: DateTime<Utc>,
    ) -> Result<SessionContext, AppError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| AppError::Persistence(rusqlite::Error::InvalidQuery))?;
        let row = connection
            .query_row(
                "SELECT s.session_id, s.user_id, s.workstation_id, u.facility_id, s.created_at_utc,
                    s.recent_auth_at_utc, s.expires_at_utc, u.active
             FROM auth_sessions s JOIN users u ON u.user_id = s.user_id
             WHERE s.token_sha256 = ?1 AND s.revoked_at_utc IS NULL",
                params![sha256_text(raw_token)],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, bool>(7)?,
                    ))
                },
            )
            .optional()?
            .ok_or(AppError::Authentication)?;
        let expires_at = parse_time(&row.6)?;
        if !row.7 || expires_at <= now {
            return Err(AppError::Authentication);
        }
        let user_id = UserId(parse_uuid(&row.1)?);
        let roles = Self::roles_for_user(&connection, user_id)?;
        Ok(SessionContext {
            session_id: SessionId(parse_uuid(&row.0)?),
            user_id,
            workstation_id: WorkstationId(parse_uuid(&row.2)?),
            facility_id: FacilityId(parse_uuid(&row.3)?),
            authenticated_at: parse_time(&row.4)?,
            recent_auth_at: parse_time(&row.5)?,
            expires_at,
            roles,
        })
    }

    pub fn create_patient_with_audit(
        &self,
        actor: &SessionContext,
        patient: &Patient,
    ) -> Result<(), AppError> {
        if !patient.validate() {
            return Err(AppError::Validation);
        }
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| AppError::Persistence(rusqlite::Error::InvalidQuery))?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute(
            "INSERT INTO patients (
                patient_id, revision, given_names, middle_names, first_surname, second_surname,
                suffix, preferred_name, date_of_birth, created_by, created_at_utc
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                patient.patient_id.to_string(),
                revision_to_i64(patient.revision)?,
                patient.name.given_names,
                patient.name.middle_names,
                patient.name.first_surname,
                patient.name.second_surname,
                patient.name.suffix,
                patient.name.preferred_name,
                patient.date_of_birth.to_string(),
                patient.created_by.to_string(),
                Utc::now().to_rfc3339(),
            ],
        )?;
        if let Some(address) = &patient.address {
            transaction.execute(
                "INSERT INTO patient_addresses (
                    patient_address_id, patient_id, line1, line2, municipality, region, postal_code, country_code
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    Uuid::new_v4().to_string(), patient.patient_id.to_string(), address.line1, address.line2,
                    address.municipality, address.region, address.postal_code, address.country_code,
                ],
            )?;
        }
        for identifier in &patient.external_identifiers {
            transaction.execute(
                "INSERT INTO external_identifiers (
                    external_identifier_id, patient_id, identifier_type, assigning_authority, identifier_value
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    Uuid::new_v4().to_string(), patient.patient_id.to_string(), identifier.identifier_type,
                    identifier.assigning_authority, identifier.value,
                ],
            )?;
        }
        append_audit(
            &transaction,
            actor,
            &AuditDraft {
                action: "PATIENT_CREATED",
                entity_type: "PATIENT",
                entity_id: &patient.patient_id.to_string(),
                entity_revision: Some(patient.revision),
                outcome: "SUCCEEDED",
                correlation_id: Uuid::new_v4(),
                metadata_json: r#"{"changedFields":["identity","demographics","address","externalIdentifiers"]}"#,
            },
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn get_patient_by_id(&self, patient_id: PatientId) -> Result<Patient, AppError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| AppError::Persistence(rusqlite::Error::InvalidQuery))?;
        let row = connection.query_row(
            "SELECT revision, given_names, middle_names, first_surname, second_surname, suffix, preferred_name, date_of_birth, created_by
             FROM patients WHERE patient_id = ?1",
            params![patient_id.to_string()],
            |row| Ok((
                row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?, row.get::<_, Option<String>>(4)?, row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?, row.get::<_, String>(7)?, row.get::<_, String>(8)?,
            )),
        ).optional()?.ok_or(AppError::NotFound)?;
        let address = connection.query_row(
            "SELECT line1, line2, municipality, region, postal_code, country_code FROM patient_addresses WHERE patient_id = ?1",
            params![patient_id.to_string()],
            |row| Ok(PatientAddress {
                line1: row.get(0)?, line2: row.get(1)?, municipality: row.get(2)?, region: row.get(3)?,
                postal_code: row.get(4)?, country_code: row.get(5)?,
            }),
        ).optional()?;
        let mut statement = connection.prepare(
            "SELECT identifier_type, assigning_authority, identifier_value FROM external_identifiers WHERE patient_id = ?1 ORDER BY identifier_type, assigning_authority",
        )?;
        let identifiers = statement
            .query_map(params![patient_id.to_string()], |row| {
                Ok(ExternalIdentifier {
                    identifier_type: row.get(0)?,
                    assigning_authority: row.get(1)?,
                    value: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Patient {
            patient_id,
            revision: revision_from_i64(row.0)?,
            name: PatientName {
                given_names: row.1,
                middle_names: row.2,
                first_surname: row.3,
                second_surname: row.4,
                suffix: row.5,
                preferred_name: row.6,
            },
            date_of_birth: chrono::NaiveDate::from_str(&row.7).map_err(|_| AppError::Validation)?,
            address,
            external_identifiers: identifiers,
            created_by: UserId(parse_uuid(&row.8)?),
        })
    }

    pub fn create_encounter_with_audit(
        &self,
        actor: &SessionContext,
        encounter: &ImmunizationEncounter,
    ) -> Result<(), AppError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| AppError::Persistence(rusqlite::Error::InvalidQuery))?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let now = Utc::now().to_rfc3339();
        transaction.execute(
            "INSERT INTO immunization_encounters (
                encounter_id, patient_id, facility_id, responsible_professional_id, state, revision, created_at_utc, updated_at_utc
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![
                encounter.encounter_id.to_string(), encounter.patient_id.to_string(), encounter.facility_id.to_string(),
                encounter.responsible_professional_id.to_string(), encounter.state.code(), revision_to_i64(encounter.revision)?, now,
            ],
        )?;
        append_audit(
            &transaction,
            actor,
            &AuditDraft {
                action: "ENCOUNTER_CREATED",
                entity_type: "IMMUNIZATION_ENCOUNTER",
                entity_id: &encounter.encounter_id.to_string(),
                entity_revision: Some(encounter.revision),
                outcome: "SUCCEEDED",
                correlation_id: Uuid::new_v4(),
                metadata_json: r#"{"changedFields":["state"]}"#,
            },
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn get_encounter_by_id(
        &self,
        encounter_id: EncounterId,
    ) -> Result<ImmunizationEncounter, AppError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| AppError::Persistence(rusqlite::Error::InvalidQuery))?;
        let row = connection.query_row(
            "SELECT patient_id, facility_id, responsible_professional_id, state, revision FROM immunization_encounters WHERE encounter_id = ?1",
            params![encounter_id.to_string()],
            |row| Ok((
                row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?,
                row.get::<_, String>(3)?, row.get::<_, i64>(4)?,
            )),
        ).optional()?.ok_or(AppError::NotFound)?;
        Ok(ImmunizationEncounter {
            encounter_id,
            patient_id: PatientId(parse_uuid(&row.0)?),
            facility_id: FacilityId(parse_uuid(&row.1)?),
            responsible_professional_id: UserId(parse_uuid(&row.2)?),
            state: EncounterState::parse(&row.3)?,
            revision: revision_from_i64(row.4)?,
        })
    }

    pub fn transition_encounter_with_audit(
        &self,
        actor: &SessionContext,
        encounter_id: EncounterId,
        expected_revision: u64,
        target: EncounterState,
    ) -> Result<ImmunizationEncounter, AppError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| AppError::Persistence(rusqlite::Error::InvalidQuery))?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let row = transaction.query_row(
            "SELECT patient_id, facility_id, responsible_professional_id, state, revision FROM immunization_encounters WHERE encounter_id = ?1",
            params![encounter_id.to_string()],
            |row| Ok((
                row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?,
                row.get::<_, String>(3)?, row.get::<_, i64>(4)?,
            )),
        ).optional()?.ok_or(AppError::NotFound)?;
        if revision_from_i64(row.4)? != expected_revision {
            return Err(AppError::StaleRevision);
        }
        let current_state = EncounterState::parse(&row.3)?;
        current_state.transition_to(target)?;
        let next_revision = expected_revision + 1;
        let updated = transaction.execute(
            "UPDATE immunization_encounters SET state = ?1, revision = ?2, updated_at_utc = ?3 WHERE encounter_id = ?4 AND revision = ?5",
            params![target.code(), revision_to_i64(next_revision)?, Utc::now().to_rfc3339(), encounter_id.to_string(), revision_to_i64(expected_revision)?],
        )?;
        if updated != 1 {
            return Err(AppError::StaleRevision);
        }
        append_audit(
            &transaction,
            actor,
            &AuditDraft {
                action: "ENCOUNTER_STATE_TRANSITIONED",
                entity_type: "IMMUNIZATION_ENCOUNTER",
                entity_id: &encounter_id.to_string(),
                entity_revision: Some(next_revision),
                outcome: "SUCCEEDED",
                correlation_id: Uuid::new_v4(),
                metadata_json: r#"{"changedFields":["state"]}"#,
            },
        )?;
        transaction.commit()?;
        Ok(ImmunizationEncounter {
            encounter_id,
            patient_id: PatientId(parse_uuid(&row.0)?),
            facility_id: FacilityId(parse_uuid(&row.1)?),
            responsible_professional_id: UserId(parse_uuid(&row.2)?),
            state: target,
            revision: next_revision,
        })
    }

    pub fn audit_event_count_value(&self) -> Result<u64, AppError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| AppError::Persistence(rusqlite::Error::InvalidQuery))?;
        let count: i64 =
            connection.query_row("SELECT COUNT(*) FROM audit_events", [], |row| row.get(0))?;
        u64::try_from(count).map_err(|_| AppError::Validation)
    }

    pub fn append_audit_event(
        &self,
        actor: &SessionContext,
        draft: &AuditDraft<'_>,
    ) -> Result<(), AppError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| AppError::Persistence(rusqlite::Error::InvalidQuery))?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        append_audit(&transaction, actor, draft)?;
        transaction.commit()?;
        Ok(())
    }

    #[cfg(test)]
    pub fn patient_count(&self) -> Result<u64, AppError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| AppError::Persistence(rusqlite::Error::InvalidQuery))?;
        let count: i64 =
            connection.query_row("SELECT COUNT(*) FROM patients", [], |row| row.get(0))?;
        u64::try_from(count).map_err(|_| AppError::Validation)
    }

    #[cfg(test)]
    pub fn execute_test_sql(&self, sql: &str) -> Result<(), AppError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| AppError::Persistence(rusqlite::Error::InvalidQuery))?;
        connection.execute_batch(sql)?;
        Ok(())
    }

    #[cfg(test)]
    pub fn audit_action_count(&self, action: &str) -> Result<u64, AppError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| AppError::Persistence(rusqlite::Error::InvalidQuery))?;
        let count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM audit_events WHERE action_code = ?1",
            params![action],
            |row| row.get(0),
        )?;
        u64::try_from(count).map_err(|_| AppError::Validation)
    }
}

impl PatientRepository for Database {
    fn create_patient(&self, actor: &SessionContext, patient: &Patient) -> Result<(), AppError> {
        self.create_patient_with_audit(actor, patient)
    }

    fn get_patient(&self, patient_id: PatientId) -> Result<Patient, AppError> {
        self.get_patient_by_id(patient_id)
    }
}

impl EncounterRepository for Database {
    fn create_encounter(
        &self,
        actor: &SessionContext,
        encounter: &ImmunizationEncounter,
    ) -> Result<(), AppError> {
        self.create_encounter_with_audit(actor, encounter)
    }

    fn get_encounter(&self, encounter_id: EncounterId) -> Result<ImmunizationEncounter, AppError> {
        self.get_encounter_by_id(encounter_id)
    }
}

impl AuditRepository for Database {
    fn audit_event_count(&self) -> Result<u64, AppError> {
        self.audit_event_count_value()
    }
}

impl EncryptedSnapshotSource for Database {
    fn write_encrypted_snapshot(
        &self,
        destination: &Path,
        snapshot_key: &[u8],
    ) -> Result<RestoreSummary, AppError> {
        #[cfg(feature = "sqlcipher")]
        {
            use std::time::Duration as StdDuration;

            if destination.exists() {
                return Err(AppError::Validation);
            }
            let source = self
                .connection
                .lock()
                .map_err(|_| AppError::Persistence(rusqlite::Error::InvalidQuery))?;
            let mut destination_connection = Connection::open(destination)?;
            Self::apply_sqlcipher_key(&destination_connection, snapshot_key)?;
            destination_connection.pragma_update(None, "journal_mode", "DELETE")?;
            destination_connection.pragma_update(None, "synchronous", "FULL")?;
            {
                let backup = rusqlite::backup::Backup::new(&source, &mut destination_connection)?;
                backup.run_to_completion(8, StdDuration::from_millis(10), None)?;
            }
            destination_connection.pragma_update(None, "journal_mode", "DELETE")?;
            drop(destination_connection);
            Self::validate_encrypted_file(destination, snapshot_key)
        }
        #[cfg(not(feature = "sqlcipher"))]
        {
            let _ = (destination, snapshot_key);
            Err(AppError::Configuration)
        }
    }
}

#[cfg(feature = "sqlcipher")]
fn validated_revision_count(connection: &Connection, table: &str) -> Result<u64, AppError> {
    let sql = match table {
        "patients" => "SELECT COUNT(*), COALESCE(MIN(revision), 1) FROM patients",
        "immunization_encounters" => {
            "SELECT COUNT(*), COALESCE(MIN(revision), 1) FROM immunization_encounters"
        }
        "users" => "SELECT COUNT(*), COALESCE(MIN(revision), 1) FROM users",
        _ => return Err(AppError::BackupIntegrity),
    };
    let (count, minimum_revision): (i64, i64) = connection
        .query_row(sql, [], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|_| AppError::BackupIntegrity)?;
    if minimum_revision < 1 {
        return Err(AppError::BackupIntegrity);
    }
    u64::try_from(count).map_err(|_| AppError::BackupIntegrity)
}

#[cfg(feature = "sqlcipher")]
fn verify_audit_chain(connection: &Connection) -> Result<u64, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT occurred_at_utc, recorded_at_utc, actor_id, session_id, workstation_id,
                    facility_id, action_code, entity_type, entity_id, entity_revision, outcome,
                    correlation_id, metadata_json, previous_hash, event_hash
             FROM audit_events ORDER BY sequence",
        )
        .map_err(|_| AppError::BackupIntegrity)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, Option<i64>>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, String>(11)?,
                row.get::<_, String>(12)?,
                row.get::<_, Option<String>>(13)?,
                row.get::<_, String>(14)?,
            ))
        })
        .map_err(|_| AppError::BackupIntegrity)?;
    let mut expected_previous: Option<String> = None;
    let mut count = 0_u64;
    for row in rows {
        let (
            occurred,
            recorded,
            actor,
            session,
            workstation,
            facility,
            action,
            entity_type,
            entity_id,
            revision,
            outcome,
            correlation,
            metadata,
            previous_hash,
            event_hash,
        ) = row.map_err(|_| AppError::BackupIntegrity)?;
        if occurred != recorded || previous_hash != expected_previous {
            return Err(AppError::BackupIntegrity);
        }
        serde_json::from_str::<serde_json::Value>(&metadata)
            .map_err(|_| AppError::BackupIntegrity)?;
        let canonical = format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            expected_previous.as_deref().unwrap_or("GENESIS"),
            occurred,
            actor.as_deref().unwrap_or_default(),
            session.as_deref().unwrap_or_default(),
            workstation,
            facility,
            action,
            entity_type,
            entity_id,
            revision.map(|value| value.to_string()).unwrap_or_default(),
            outcome,
            correlation,
            metadata,
        );
        if sha256_text(&canonical) != event_hash {
            return Err(AppError::BackupIntegrity);
        }
        expected_previous = Some(event_hash);
        count = count.checked_add(1).ok_or(AppError::BackupIntegrity)?;
    }
    Ok(count)
}

fn append_audit(
    transaction: &Transaction<'_>,
    actor: &SessionContext,
    draft: &AuditDraft<'_>,
) -> Result<(), AppError> {
    let previous_hash: Option<String> = transaction
        .query_row(
            "SELECT event_hash FROM audit_events ORDER BY sequence DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let now = Utc::now();
    let canonical = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        previous_hash.as_deref().unwrap_or("GENESIS"),
        now.to_rfc3339(),
        actor.user_id,
        actor.session_id,
        actor.workstation_id,
        actor.facility_id,
        draft.action,
        draft.entity_type,
        draft.entity_id,
        draft
            .entity_revision
            .map(|value| value.to_string())
            .unwrap_or_default(),
        draft.outcome,
        draft.correlation_id,
        draft.metadata_json,
    );
    let event_hash = sha256_text(&canonical);
    transaction.execute(
        "INSERT INTO audit_events (
            audit_event_id, occurred_at_utc, recorded_at_utc, actor_id, session_id, workstation_id,
            facility_id, action_code, entity_type, entity_id, entity_revision, outcome,
            correlation_id, software_version, schema_version, metadata_json, previous_hash, event_hash
         ) VALUES (?1, ?2, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        params![
            Uuid::new_v4().to_string(), now.to_rfc3339(), actor.user_id.to_string(), actor.session_id.to_string(),
            actor.workstation_id.to_string(), actor.facility_id.to_string(), draft.action, draft.entity_type,
            draft.entity_id, draft.entity_revision.map(revision_to_i64).transpose()?, draft.outcome, draft.correlation_id.to_string(),
            env!("CARGO_PKG_VERSION"), SCHEMA_VERSION, draft.metadata_json, previous_hash, event_hash,
        ],
    )?;
    Ok(())
}

fn append_unauthenticated_audit(
    transaction: &Transaction<'_>,
    draft: &UnauthenticatedAuditDraft<'_>,
) -> Result<(), AppError> {
    let previous_hash: Option<String> = transaction
        .query_row(
            "SELECT event_hash FROM audit_events ORDER BY sequence DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let correlation_id = Uuid::new_v4();
    let actor_text = draft
        .actor_id
        .map(|value| value.to_string())
        .unwrap_or_default();
    let entity_id = if actor_text.is_empty() {
        "UNKNOWN"
    } else {
        &actor_text
    };
    let canonical = format!(
        "{}|{}|{}||{}|{}|{}|USER_ACCOUNT|{}||{}|{}|{}",
        previous_hash.as_deref().unwrap_or("GENESIS"),
        draft.now.to_rfc3339(),
        actor_text,
        draft.workstation_id,
        draft.facility_id,
        draft.action,
        entity_id,
        draft.outcome,
        correlation_id,
        draft.metadata_json,
    );
    let event_hash = sha256_text(&canonical);
    transaction.execute(
        "INSERT INTO audit_events (
            audit_event_id, occurred_at_utc, recorded_at_utc, actor_id, session_id, workstation_id,
            facility_id, action_code, entity_type, entity_id, entity_revision, outcome,
            correlation_id, software_version, schema_version, metadata_json, previous_hash, event_hash
         ) VALUES (?1, ?2, ?2, ?3, NULL, ?4, ?5, ?6, 'USER_ACCOUNT', ?7, NULL, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            Uuid::new_v4().to_string(),
            draft.now.to_rfc3339(),
            draft.actor_id.map(|value| value.to_string()),
            draft.workstation_id.to_string(),
            draft.facility_id.to_string(),
            draft.action,
            entity_id,
            draft.outcome,
            correlation_id.to_string(),
            env!("CARGO_PKG_VERSION"),
            SCHEMA_VERSION,
            draft.metadata_json,
            previous_hash,
            event_hash,
        ],
    )?;
    Ok(())
}

fn parse_uuid(value: &str) -> Result<Uuid, AppError> {
    Uuid::from_str(value).map_err(|_| AppError::Validation)
}

fn parse_time(value: &str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| AppError::Validation)
}

fn parse_optional_time(value: Option<String>) -> Result<Option<DateTime<Utc>>, AppError> {
    value.map(|value| parse_time(&value)).transpose()
}

fn sha256_text(value: &str) -> String {
    Sha256::digest(value.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn revision_to_i64(value: u64) -> Result<i64, AppError> {
    i64::try_from(value).map_err(|_| AppError::Validation)
}

fn revision_from_i64(value: i64) -> Result<u64, AppError> {
    u64::try_from(value).map_err(|_| AppError::Validation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    #[cfg(feature = "sqlcipher")]
    use std::fs;

    #[test]
    fn synthetic_mode_never_reclassifies_an_existing_unlabeled_database() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("unknown.sqlite");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute("CREATE TABLE unknown_data (value TEXT NOT NULL)", [])
            .unwrap();
        drop(connection);
        assert!(matches!(
            Database::open_synthetic(&path),
            Err(AppError::Configuration)
        ));
    }

    #[test]
    fn migration_checksum_drift_fails_closed() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("migration-drift.sqlite");
        drop(Database::open_synthetic(&path).unwrap());
        let connection = Connection::open(&path).unwrap();
        connection
            .execute(
                "UPDATE schema_migrations SET checksum = 'sha256:tampered' WHERE version = 1",
                [],
            )
            .unwrap();
        drop(connection);

        assert!(matches!(
            Database::open_synthetic(&path),
            Err(AppError::Configuration)
        ));
    }

    #[test]
    fn file_database_survives_close_and_restart() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("restart.sqlite");
        let patient_id;
        {
            let database = Database::open_synthetic(&path).unwrap();
            let now = Utc::now();
            let facility_id = FacilityId::new();
            let workstation_id = WorkstationId::new();
            database
                .create_facility_and_workstation(
                    &Facility {
                        facility_id,
                        name: "Synthetic Restart Facility".to_owned(),
                        timezone: "America/Puerto_Rico".to_owned(),
                        active: true,
                    },
                    workstation_id,
                    "SYNTHETIC-RESTART-WS",
                    now,
                )
                .unwrap();
            let user = User {
                user_id: UserId::new(),
                username: "synthetic.restart".to_owned(),
                display_name: "Synthetic Restart User".to_owned(),
                active: true,
                roles: vec![Role::ClinicalSupport],
            };
            database
                .create_user(&user, facility_id, "SYNTHETIC-NOT-A-REAL-VERIFIER", now)
                .unwrap();
            let actor = database
                .create_session(
                    &user,
                    workstation_id,
                    "synthetic-session-token-for-restart-test",
                    now,
                )
                .unwrap();
            patient_id = PatientId::new();
            database
                .create_patient_with_audit(
                    &actor,
                    &Patient {
                        patient_id,
                        revision: 1,
                        name: PatientName {
                            given_names: "Restart Synthetic".to_owned(),
                            middle_names: None,
                            first_surname: "Persistence".to_owned(),
                            second_surname: Some("Test".to_owned()),
                            suffix: None,
                            preferred_name: None,
                        },
                        date_of_birth: NaiveDate::from_ymd_opt(2001, 1, 1).unwrap(),
                        address: None,
                        external_identifiers: vec![],
                        created_by: user.user_id,
                    },
                )
                .unwrap();
        }
        let reopened = Database::open_synthetic(&path).unwrap();
        let patient = reopened.get_patient_by_id(patient_id).unwrap();
        assert_eq!(patient.name.first_surname, "Persistence");
        assert_eq!(reopened.audit_event_count_value().unwrap(), 2);
    }

    #[cfg(feature = "sqlcipher")]
    #[test]
    fn integrated_encrypted_path_preserves_constraints_revisions_audit_and_wal_confidentiality() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("integrated-encrypted.sqlite");
        let key = Zeroizing::new(vec![7_u8; 32]);
        let database = Database::create_encrypted(&path, &key).unwrap();
        {
            let connection = database.connection.lock().unwrap();
            let foreign_keys: i64 = connection
                .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
                .unwrap();
            let journal_mode: String = connection
                .query_row("PRAGMA journal_mode", [], |row| row.get(0))
                .unwrap();
            assert_eq!(foreign_keys, 1);
            assert_eq!(journal_mode.to_ascii_lowercase(), "wal");
        }

        let now = Utc::now();
        let facility_id = FacilityId::new();
        let workstation_id = WorkstationId::new();
        database
            .create_facility_and_workstation(
                &Facility {
                    facility_id,
                    name: "Synthetic Encrypted Facility".to_owned(),
                    timezone: "America/Puerto_Rico".to_owned(),
                    active: true,
                },
                workstation_id,
                "SYNTHETIC-ENCRYPTED-WS",
                now,
            )
            .unwrap();
        let user = User {
            user_id: UserId::new(),
            username: "synthetic.encrypted".to_owned(),
            display_name: "Synthetic Encrypted User".to_owned(),
            active: true,
            roles: vec![Role::VaccinatingProfessional],
        };
        database
            .create_user(&user, facility_id, "SYNTHETIC-NOT-A-VERIFIER", now)
            .unwrap();
        let actor = database
            .create_session(&user, workstation_id, "synthetic-encrypted-session", now)
            .unwrap();
        let patient_id = PatientId::new();
        database
            .create_patient_with_audit(
                &actor,
                &Patient {
                    patient_id,
                    revision: 1,
                    name: PatientName {
                        given_names: "Synthetic".to_owned(),
                        middle_names: None,
                        first_surname: "SYNTHETIC-WAL-SENTINEL".to_owned(),
                        second_surname: None,
                        suffix: None,
                        preferred_name: None,
                    },
                    date_of_birth: NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
                    address: None,
                    external_identifiers: Vec::new(),
                    created_by: user.user_id,
                },
            )
            .unwrap();
        let encounter_id = EncounterId::new();
        database
            .create_encounter_with_audit(
                &actor,
                &ImmunizationEncounter {
                    encounter_id,
                    patient_id,
                    facility_id,
                    responsible_professional_id: user.user_id,
                    state: EncounterState::Draft,
                    revision: 1,
                },
            )
            .unwrap();
        let transitioned = database
            .transition_encounter_with_audit(
                &actor,
                encounter_id,
                1,
                EncounterState::ReadyToAdminister,
            )
            .unwrap();
        assert_eq!(transitioned.revision, 2);
        assert!(matches!(
            database.transition_encounter_with_audit(
                &actor,
                encounter_id,
                1,
                EncounterState::AdministeredPendingDocumentation,
            ),
            Err(AppError::StaleRevision)
        ));
        assert_eq!(database.audit_event_count_value().unwrap(), 4);

        for entry in fs::read_dir(temp.path()).unwrap() {
            let bytes = fs::read(entry.unwrap().path()).unwrap();
            assert!(
                !bytes
                    .windows(22)
                    .any(|window| window == b"SYNTHETIC-WAL-SENTINEL")
            );
        }
        drop(database);
        let reopened = Database::open_encrypted(&path, &key).unwrap();
        assert_eq!(
            reopened.get_encounter_by_id(encounter_id).unwrap().revision,
            2
        );
        assert_eq!(reopened.audit_event_count_value().unwrap(), 4);
    }
}
