use chrono::{DateTime, Utc};

use std::path::Path;

use uuid::Uuid;

use crate::adapters::{AuditDraft, Database};
use crate::domain::{
    EncounterId, EncounterState, ImmunizationEncounter, Patient, Permission, SessionContext,
};
use crate::error::AppError;
use crate::ports::{BackupReceipt, BackupService, SecretStore, StagedRestore};

pub struct AuthorizationService;

impl AuthorizationService {
    pub fn require(session: &SessionContext, permission: Permission) -> Result<(), AppError> {
        if session.has_permission(permission) {
            Ok(())
        } else {
            Err(AppError::Authorization)
        }
    }
}

pub struct PatientService<'a> {
    database: &'a Database,
}

impl<'a> PatientService<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn create(&self, actor: &SessionContext, patient: &Patient) -> Result<(), AppError> {
        if let Err(error) = AuthorizationService::require(actor, Permission::PatientCreate) {
            self.audit_failure(actor, patient, "DENIED", &error)?;
            return Err(error);
        }
        match self.database.create_patient_with_audit(actor, patient) {
            Ok(()) => Ok(()),
            Err(error) => {
                self.audit_failure(actor, patient, "FAILED", &error)?;
                Err(error)
            }
        }
    }

    fn audit_failure(
        &self,
        actor: &SessionContext,
        patient: &Patient,
        outcome: &str,
        error: &AppError,
    ) -> Result<(), AppError> {
        let entity_id = patient.patient_id.to_string();
        let metadata = format!(r#"{{"errorCode":"{}"}}"#, safe_error_code(error));
        self.database.append_audit_event(
            actor,
            &AuditDraft {
                action: "PATIENT_CREATE_ATTEMPT",
                entity_type: "PATIENT",
                entity_id: &entity_id,
                entity_revision: Some(patient.revision),
                outcome,
                correlation_id: Uuid::new_v4(),
                metadata_json: &metadata,
            },
        )
    }

    pub fn get(
        &self,
        actor: &SessionContext,
        patient_id: crate::domain::PatientId,
    ) -> Result<Patient, AppError> {
        AuthorizationService::require(actor, Permission::PatientRead)?;
        self.database.get_patient_by_id(patient_id)
    }
}

pub struct EncounterService<'a> {
    database: &'a Database,
}

pub struct BackupApplicationService<'a> {
    database: &'a Database,
    backup: &'a dyn BackupService,
    secret_store: &'a dyn SecretStore,
}

impl<'a> BackupApplicationService<'a> {
    pub fn new(
        database: &'a Database,
        backup: &'a dyn BackupService,
        secret_store: &'a dyn SecretStore,
    ) -> Self {
        Self {
            database,
            backup,
            secret_store,
        }
    }

    pub fn create_manual_backup(
        &self,
        actor: &SessionContext,
        destination: &Path,
        recovery_secret: &[u8],
        _now: DateTime<Utc>,
    ) -> Result<BackupReceipt, AppError> {
        let operation_id = Uuid::new_v4();
        if let Err(error) = AuthorizationService::require(actor, Permission::BackupManage) {
            self.audit_error(actor, operation_id, "BACKUP_FAILED", "DENIED", &error)?;
            return Err(error);
        }
        self.audit(actor, operation_id, "BACKUP_STARTED", "SUCCEEDED", "{}")?;
        match self
            .backup
            .create_encrypted_backup(self.database, destination, recovery_secret)
        {
            Ok(receipt) => {
                let metadata = format!(
                    r#"{{"backupId":"{}","formatVersion":{},"encryptedSizeBytes":{}}}"#,
                    receipt.backup_id, receipt.format_version, receipt.encrypted_size_bytes
                );
                self.audit(
                    actor,
                    operation_id,
                    "BACKUP_SUCCEEDED",
                    "SUCCEEDED",
                    &metadata,
                )?;
                Ok(receipt)
            }
            Err(error) => {
                self.audit_error(actor, operation_id, "BACKUP_FAILED", "FAILED", &error)?;
                Err(error)
            }
        }
    }

    pub fn stage_restore(
        &self,
        actor: &SessionContext,
        backup_path: &Path,
        staging_directory: &Path,
        recovery_secret: &[u8],
        now: DateTime<Utc>,
    ) -> Result<StagedRestore, AppError> {
        let operation_id = Uuid::new_v4();
        if let Err(error) = self.authorize_restore(actor, now) {
            self.audit_error(actor, operation_id, "RESTORE_FAILED", "DENIED", &error)?;
            return Err(error);
        }
        self.audit(actor, operation_id, "RESTORE_STARTED", "SUCCEEDED", "{}")?;
        match self
            .backup
            .stage_restore(backup_path, staging_directory, recovery_secret)
        {
            Ok(staged) => {
                let metadata = format!(
                    r#"{{"backupId":"{}","schemaVersion":{},"auditEventCount":{}}}"#,
                    staged.backup_id,
                    staged.summary.schema_version,
                    staged.summary.audit_event_count
                );
                self.audit(
                    actor,
                    operation_id,
                    "RESTORE_VALIDATED",
                    "SUCCEEDED",
                    &metadata,
                )?;
                Ok(staged)
            }
            Err(error) => {
                self.audit_error(actor, operation_id, "RESTORE_FAILED", "FAILED", &error)?;
                Err(error)
            }
        }
    }

    pub fn cutover(
        &self,
        actor: &SessionContext,
        staged: StagedRestore,
        destination: &Path,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let operation_id = staged.backup_id;
        if let Err(error) = self.authorize_restore(actor, now) {
            self.audit_error(actor, operation_id, "RESTORE_FAILED", "DENIED", &error)?;
            return Err(error);
        }
        match self.backup.cutover(staged, destination, self.secret_store) {
            Ok(()) => self.audit(
                actor,
                operation_id,
                "RESTORE_CUTOVER_CONFIRMED",
                "SUCCEEDED",
                "{}",
            ),
            Err(error) => {
                self.audit_error(actor, operation_id, "RESTORE_FAILED", "FAILED", &error)?;
                Err(error)
            }
        }
    }

    fn authorize_restore(
        &self,
        actor: &SessionContext,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        AuthorizationService::require(actor, Permission::BackupManage)?;
        crate::application::auth::AuthService::require_recent_auth(actor, now)?;
        Ok(())
    }

    fn audit_error(
        &self,
        actor: &SessionContext,
        operation_id: Uuid,
        action: &str,
        outcome: &str,
        error: &AppError,
    ) -> Result<(), AppError> {
        let metadata = format!(r#"{{"errorCode":"{}"}}"#, safe_error_code(error));
        self.audit(actor, operation_id, action, outcome, &metadata)
    }

    fn audit(
        &self,
        actor: &SessionContext,
        operation_id: Uuid,
        action: &str,
        outcome: &str,
        metadata_json: &str,
    ) -> Result<(), AppError> {
        let operation_id = operation_id.to_string();
        self.database.append_audit_event(
            actor,
            &AuditDraft {
                action,
                entity_type: "BACKUP_OPERATION",
                entity_id: &operation_id,
                entity_revision: None,
                outcome,
                correlation_id: Uuid::new_v4(),
                metadata_json,
            },
        )
    }
}

impl<'a> EncounterService<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn create(
        &self,
        actor: &SessionContext,
        encounter: &ImmunizationEncounter,
    ) -> Result<(), AppError> {
        if let Err(error) = AuthorizationService::require(actor, Permission::EncounterCreate) {
            self.audit_transition_failure(
                actor,
                encounter.encounter_id,
                Some(encounter.revision),
                "ENCOUNTER_CREATE_ATTEMPT",
                "DENIED",
                &error,
            )?;
            return Err(error);
        }
        match self.database.create_encounter_with_audit(actor, encounter) {
            Ok(()) => Ok(()),
            Err(error) => {
                self.audit_transition_failure(
                    actor,
                    encounter.encounter_id,
                    Some(encounter.revision),
                    "ENCOUNTER_CREATE_ATTEMPT",
                    "FAILED",
                    &error,
                )?;
                Err(error)
            }
        }
    }

    pub fn transition(
        &self,
        actor: &SessionContext,
        encounter_id: EncounterId,
        expected_revision: u64,
        target: EncounterState,
        now: DateTime<Utc>,
    ) -> Result<ImmunizationEncounter, AppError> {
        let current = self.database.get_encounter_by_id(encounter_id)?;
        let permission = current.state.required_permission_for_transition(target);
        if let Err(error) = AuthorizationService::require(actor, permission) {
            self.audit_transition_failure(
                actor,
                encounter_id,
                Some(expected_revision),
                "ENCOUNTER_TRANSITION_ATTEMPT",
                "DENIED",
                &error,
            )?;
            return Err(error);
        }
        let requires_recent_auth = matches!(
            target,
            EncounterState::AdministeredPendingDocumentation
                | EncounterState::Finalized
                | EncounterState::RegistryReady
                | EncounterState::Corrected
                | EncounterState::Voided
        );
        if requires_recent_auth
            && let Err(error) =
                crate::application::auth::AuthService::require_recent_auth(actor, now)
        {
            self.audit_transition_failure(
                actor,
                encounter_id,
                Some(expected_revision),
                "ENCOUNTER_TRANSITION_ATTEMPT",
                "DENIED",
                &error,
            )?;
            return Err(error);
        }
        match self.database.transition_encounter_with_audit(
            actor,
            encounter_id,
            expected_revision,
            target,
        ) {
            Ok(encounter) => Ok(encounter),
            Err(error) => {
                self.audit_transition_failure(
                    actor,
                    encounter_id,
                    Some(expected_revision),
                    "ENCOUNTER_TRANSITION_ATTEMPT",
                    "FAILED",
                    &error,
                )?;
                Err(error)
            }
        }
    }

    fn audit_transition_failure(
        &self,
        actor: &SessionContext,
        encounter_id: EncounterId,
        revision: Option<u64>,
        action: &str,
        outcome: &str,
        error: &AppError,
    ) -> Result<(), AppError> {
        let entity_id = encounter_id.to_string();
        let metadata = format!(r#"{{"errorCode":"{}"}}"#, safe_error_code(error));
        self.database.append_audit_event(
            actor,
            &AuditDraft {
                action,
                entity_type: "IMMUNIZATION_ENCOUNTER",
                entity_id: &entity_id,
                entity_revision: revision,
                outcome,
                correlation_id: Uuid::new_v4(),
                metadata_json: &metadata,
            },
        )
    }
}

fn safe_error_code(error: &AppError) -> &'static str {
    match error {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "sqlcipher")]
    use crate::adapters::DatabaseKeyLifecycle;
    use crate::adapters::{EncryptedBackupService, FakeSecretStore};
    use crate::domain::{Facility, FacilityId, Role, User, UserId, WorkstationId};

    fn actor_fixture(role: Role) -> (tempfile::TempDir, Database, SessionContext) {
        let temp = tempfile::tempdir().unwrap();
        let database = Database::open_synthetic(temp.path().join("foundation.sqlite")).unwrap();
        let actor = actor_for_database(&database, role);
        (temp, database, actor)
    }

    #[cfg(feature = "sqlcipher")]
    fn encrypted_actor_fixture(
        role: Role,
    ) -> (tempfile::TempDir, FakeSecretStore, Database, SessionContext) {
        let temp = tempfile::tempdir().unwrap();
        let secret_store = FakeSecretStore::new();
        let database = DatabaseKeyLifecycle::new(&secret_store)
            .create_encrypted_database(&temp.path().join("foundation.sqlite"))
            .unwrap();
        let actor = actor_for_database(&database, role);
        (temp, secret_store, database, actor)
    }

    fn actor_for_database(database: &Database, role: Role) -> SessionContext {
        let now = Utc::now();
        let facility_id = FacilityId::new();
        let workstation_id = WorkstationId::new();
        database
            .create_facility_and_workstation(
                &Facility {
                    facility_id,
                    name: "Synthetic Backup Facility".to_owned(),
                    timezone: "America/Puerto_Rico".to_owned(),
                    active: true,
                },
                workstation_id,
                "SYNTHETIC-BACKUP-WS",
                now,
            )
            .unwrap();
        let user = User {
            user_id: UserId::new(),
            username: format!("synthetic.{}", role.code().to_lowercase()),
            display_name: "Synthetic Backup User".to_owned(),
            active: true,
            roles: vec![role],
        };
        database
            .create_user(&user, facility_id, "SYNTHETIC-NOT-A-VERIFIER", now)
            .unwrap();
        database
            .create_session(&user, workstation_id, "synthetic-backup-session-token", now)
            .unwrap()
    }

    #[cfg(feature = "sqlcipher")]
    #[test]
    fn authorized_manual_backup_is_audited() {
        let (temp, secret_store, database, actor) =
            encrypted_actor_fixture(Role::FacilityAdministrator);
        let backup_path = temp.path().join("manual.avxbak");
        let before = database.audit_event_count_value().unwrap();
        let service = EncryptedBackupService;
        let receipt = BackupApplicationService::new(&database, &service, &secret_store)
            .create_manual_backup(
                &actor,
                &backup_path,
                b"synthetic-recovery-passphrase",
                Utc::now(),
            )
            .unwrap();
        assert_eq!(receipt.format_version, 2);
        assert_eq!(database.audit_event_count_value().unwrap(), before + 2);
        assert_eq!(database.audit_action_count("BACKUP_STARTED").unwrap(), 1);
        assert_eq!(database.audit_action_count("BACKUP_SUCCEEDED").unwrap(), 1);
    }

    #[cfg(feature = "sqlcipher")]
    #[test]
    fn authorized_restore_validation_and_cutover_emit_required_audit_events() {
        let (temp, secret_store, database, actor) =
            encrypted_actor_fixture(Role::FacilityAdministrator);
        let service = EncryptedBackupService;
        let application = BackupApplicationService::new(&database, &service, &secret_store);
        let backup_path = temp.path().join("manual.avxbak");
        let destination = temp.path().join("restored.sqlite");
        application
            .create_manual_backup(
                &actor,
                &backup_path,
                b"synthetic-recovery-passphrase",
                Utc::now(),
            )
            .unwrap();
        let staged = application
            .stage_restore(
                &actor,
                &backup_path,
                temp.path(),
                b"synthetic-recovery-passphrase",
                Utc::now(),
            )
            .unwrap();
        application
            .cutover(&actor, staged, &destination, Utc::now())
            .unwrap();
        assert_eq!(database.audit_action_count("RESTORE_STARTED").unwrap(), 1);
        assert_eq!(database.audit_action_count("RESTORE_VALIDATED").unwrap(), 1);
        assert_eq!(
            database
                .audit_action_count("RESTORE_CUTOVER_CONFIRMED")
                .unwrap(),
            1
        );
        assert!(
            DatabaseKeyLifecycle::new(&secret_store)
                .open_encrypted_database(&destination)
                .is_ok()
        );
    }

    #[test]
    fn unauthorized_manual_backup_is_denied_and_audited() {
        let (temp, database, actor) = actor_fixture(Role::ClinicalSupport);
        let backup_path = temp.path().join("manual.avxbak");
        let before = database.audit_event_count_value().unwrap();
        let service = EncryptedBackupService;
        let secret_store = FakeSecretStore::new();
        assert!(matches!(
            BackupApplicationService::new(&database, &service, &secret_store).create_manual_backup(
                &actor,
                &backup_path,
                b"synthetic-recovery-passphrase",
                Utc::now(),
            ),
            Err(AppError::Authorization)
        ));
        assert!(!backup_path.exists());
        assert_eq!(database.audit_event_count_value().unwrap(), before + 1);
        assert_eq!(database.audit_action_count("BACKUP_FAILED").unwrap(), 1);
    }

    #[test]
    fn forged_restore_request_is_denied_before_file_access_and_audited() {
        let (temp, database, actor) = actor_fixture(Role::ClinicalSupport);
        let service = EncryptedBackupService;
        let secret_store = FakeSecretStore::new();
        let before = database.audit_event_count_value().unwrap();
        assert!(matches!(
            BackupApplicationService::new(&database, &service, &secret_store).stage_restore(
                &actor,
                &temp.path().join("does-not-exist.avxbak"),
                temp.path(),
                b"synthetic-recovery-passphrase",
                Utc::now(),
            ),
            Err(AppError::Authorization)
        ));
        assert_eq!(database.audit_event_count_value().unwrap(), before + 1);
        assert_eq!(database.audit_action_count("RESTORE_FAILED").unwrap(), 1);
    }

    #[test]
    fn restore_requires_recent_reauthentication() {
        let (temp, database, mut actor) = actor_fixture(Role::FacilityAdministrator);
        actor.recent_auth_at = Utc::now() - chrono::Duration::minutes(10);
        let service = EncryptedBackupService;
        let secret_store = FakeSecretStore::new();
        assert!(matches!(
            BackupApplicationService::new(&database, &service, &secret_store).stage_restore(
                &actor,
                &temp.path().join("does-not-exist.avxbak"),
                temp.path(),
                b"synthetic-recovery-passphrase",
                Utc::now(),
            ),
            Err(AppError::Authentication)
        ));
        assert_eq!(database.audit_action_count("RESTORE_FAILED").unwrap(), 1);
    }
}
