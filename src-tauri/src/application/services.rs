use chrono::{DateTime, Utc};

use std::path::Path;

use uuid::Uuid;

use crate::adapters::{AuditDraft, Database};
use crate::domain::{
    EncounterId, EncounterState, ImmunizationEncounter, Patient, Permission, SessionContext,
};
use crate::error::AppError;
use crate::ports::{BackupReceipt, BackupService, StagedRestore};

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
}

impl<'a> BackupApplicationService<'a> {
    pub fn new(database: &'a Database, backup: &'a dyn BackupService) -> Self {
        Self { database, backup }
    }

    pub fn create_manual_backup(
        &self,
        actor: &SessionContext,
        database_path: &Path,
        destination: &Path,
        recovery_passphrase: &[u8],
        now: DateTime<Utc>,
    ) -> Result<BackupReceipt, AppError> {
        self.authorize(actor, now, "BACKUP_CREATE")?;
        match self
            .backup
            .create_encrypted_backup(database_path, destination, recovery_passphrase)
        {
            Ok(receipt) => {
                let metadata = format!(
                    r#"{{"formatVersion":{},"encryptedSizeBytes":{}}}"#,
                    receipt.format_version, receipt.encrypted_size_bytes
                );
                self.audit(actor, "BACKUP_CREATE", "SUCCEEDED", &metadata)?;
                Ok(receipt)
            }
            Err(error) => {
                self.audit_error(actor, "BACKUP_CREATE", "FAILED", &error)?;
                Err(error)
            }
        }
    }

    pub fn stage_restore(
        &self,
        actor: &SessionContext,
        backup_path: &Path,
        staging_directory: &Path,
        recovery_passphrase: &[u8],
        now: DateTime<Utc>,
    ) -> Result<StagedRestore, AppError> {
        self.authorize(actor, now, "BACKUP_RESTORE_STAGE")?;
        match self
            .backup
            .stage_restore(backup_path, staging_directory, recovery_passphrase)
        {
            Ok(staged) => {
                self.audit(actor, "BACKUP_RESTORE_STAGE", "SUCCEEDED", "{}")?;
                Ok(staged)
            }
            Err(error) => {
                self.audit_error(actor, "BACKUP_RESTORE_STAGE", "FAILED", &error)?;
                Err(error)
            }
        }
    }

    pub fn cutover(
        &self,
        actor: &SessionContext,
        staged: &StagedRestore,
        destination: &Path,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        self.authorize(actor, now, "BACKUP_RESTORE_CUTOVER")?;
        match self.backup.cutover(staged, destination) {
            Ok(()) => self.audit(actor, "BACKUP_RESTORE_CUTOVER", "SUCCEEDED", "{}"),
            Err(error) => {
                self.audit_error(actor, "BACKUP_RESTORE_CUTOVER", "FAILED", &error)?;
                Err(error)
            }
        }
    }

    fn authorize(
        &self,
        actor: &SessionContext,
        now: DateTime<Utc>,
        action: &str,
    ) -> Result<(), AppError> {
        if let Err(error) = AuthorizationService::require(actor, Permission::BackupManage) {
            self.audit_error(actor, action, "DENIED", &error)?;
            return Err(error);
        }
        if let Err(error) = crate::application::auth::AuthService::require_recent_auth(actor, now) {
            self.audit_error(actor, action, "DENIED", &error)?;
            return Err(error);
        }
        Ok(())
    }

    fn audit_error(
        &self,
        actor: &SessionContext,
        action: &str,
        outcome: &str,
        error: &AppError,
    ) -> Result<(), AppError> {
        let metadata = format!(r#"{{"errorCode":"{}"}}"#, safe_error_code(error));
        self.audit(actor, action, outcome, &metadata)
    }

    fn audit(
        &self,
        actor: &SessionContext,
        action: &str,
        outcome: &str,
        metadata_json: &str,
    ) -> Result<(), AppError> {
        let operation_id = Uuid::new_v4().to_string();
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
    use crate::adapters::EncryptedBackupService;
    use crate::domain::{Facility, FacilityId, Role, User, UserId, WorkstationId};

    fn actor_fixture(role: Role) -> (tempfile::TempDir, Database, SessionContext) {
        let temp = tempfile::tempdir().unwrap();
        let database = Database::open_synthetic(temp.path().join("foundation.sqlite")).unwrap();
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
        let actor = database
            .create_session(&user, workstation_id, "synthetic-backup-session-token", now)
            .unwrap();
        (temp, database, actor)
    }

    #[test]
    fn authorized_manual_backup_is_audited() {
        let (temp, database, actor) = actor_fixture(Role::FacilityAdministrator);
        let backup_path = temp.path().join("manual.avxbak");
        let before = database.audit_event_count_value().unwrap();
        let service = EncryptedBackupService;
        let receipt = BackupApplicationService::new(&database, &service)
            .create_manual_backup(
                &actor,
                database.path(),
                &backup_path,
                b"synthetic-recovery-passphrase",
                Utc::now(),
            )
            .unwrap();
        assert_eq!(receipt.format_version, 1);
        assert_eq!(database.audit_event_count_value().unwrap(), before + 1);
    }

    #[test]
    fn unauthorized_manual_backup_is_denied_and_audited() {
        let (temp, database, actor) = actor_fixture(Role::ClinicalSupport);
        let backup_path = temp.path().join("manual.avxbak");
        let before = database.audit_event_count_value().unwrap();
        let service = EncryptedBackupService;
        assert!(matches!(
            BackupApplicationService::new(&database, &service).create_manual_backup(
                &actor,
                database.path(),
                &backup_path,
                b"synthetic-recovery-passphrase",
                Utc::now(),
            ),
            Err(AppError::Authorization)
        ));
        assert!(!backup_path.exists());
        assert_eq!(database.audit_event_count_value().unwrap(), before + 1);
    }
}
