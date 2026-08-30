use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{FacilityId, SessionId, UserId, WorkstationId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Role {
    VaccinatingProfessional,
    ClinicalSupport,
    FacilityAdministrator,
    AuditorPrivacyReviewer,
}

impl Role {
    pub const ALL: [Self; 4] = [
        Self::VaccinatingProfessional,
        Self::ClinicalSupport,
        Self::FacilityAdministrator,
        Self::AuditorPrivacyReviewer,
    ];

    pub const fn code(self) -> &'static str {
        match self {
            Self::VaccinatingProfessional => "VACCINATING_PROFESSIONAL",
            Self::ClinicalSupport => "CLINICAL_SUPPORT",
            Self::FacilityAdministrator => "FACILITY_ADMINISTRATOR",
            Self::AuditorPrivacyReviewer => "AUDITOR_PRIVACY_REVIEWER",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|role| role.code() == value)
    }
}

#[derive(Debug, Clone)]
pub struct SessionContext {
    pub user_id: UserId,
    pub session_id: SessionId,
    pub workstation_id: WorkstationId,
    pub facility_id: FacilityId,
    pub roles: Vec<Role>,
    pub authenticated_at: DateTime<Utc>,
    pub recent_auth_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl SessionContext {
    pub fn has_permission(&self, permission: Permission) -> bool {
        permissions_for_roles(&self.roles).contains(&permission)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Permission {
    PatientRead,
    PatientCreate,
    EncounterCreate,
    EncounterEditDraft,
    EncounterMarkReady,
    AdministrationConfirm,
    EncounterFinalize,
    EncounterCorrect,
    EncounterVoid,
    RegistryPrepare,
    RegistryAuthorizeExport,
    UserManage,
    FacilityManage,
    BackupManage,
    AuditRead,
}

impl Permission {
    pub const ALL: [Self; 15] = [
        Self::PatientRead,
        Self::PatientCreate,
        Self::EncounterCreate,
        Self::EncounterEditDraft,
        Self::EncounterMarkReady,
        Self::AdministrationConfirm,
        Self::EncounterFinalize,
        Self::EncounterCorrect,
        Self::EncounterVoid,
        Self::RegistryPrepare,
        Self::RegistryAuthorizeExport,
        Self::UserManage,
        Self::FacilityManage,
        Self::BackupManage,
        Self::AuditRead,
    ];

    pub const fn code(self) -> &'static str {
        match self {
            Self::PatientRead => "PATIENT_READ",
            Self::PatientCreate => "PATIENT_CREATE",
            Self::EncounterCreate => "ENCOUNTER_CREATE",
            Self::EncounterEditDraft => "ENCOUNTER_EDIT_DRAFT",
            Self::EncounterMarkReady => "ENCOUNTER_MARK_READY",
            Self::AdministrationConfirm => "ADMINISTRATION_CONFIRM",
            Self::EncounterFinalize => "ENCOUNTER_FINALIZE",
            Self::EncounterCorrect => "ENCOUNTER_CORRECT",
            Self::EncounterVoid => "ENCOUNTER_VOID",
            Self::RegistryPrepare => "REGISTRY_PREPARE",
            Self::RegistryAuthorizeExport => "REGISTRY_AUTHORIZE_EXPORT",
            Self::UserManage => "USER_MANAGE",
            Self::FacilityManage => "FACILITY_MANAGE",
            Self::BackupManage => "BACKUP_MANAGE",
            Self::AuditRead => "AUDIT_READ",
        }
    }
}

pub fn permissions_for_roles(roles: &[Role]) -> HashSet<Permission> {
    roles
        .iter()
        .flat_map(|role| permissions_for_role(*role).iter().copied())
        .collect()
}

pub fn permissions_for_role(role: Role) -> &'static [Permission] {
    use Permission::*;
    match role {
        Role::VaccinatingProfessional => &[
            PatientRead,
            PatientCreate,
            EncounterCreate,
            EncounterEditDraft,
            EncounterMarkReady,
            AdministrationConfirm,
            EncounterFinalize,
            EncounterCorrect,
            EncounterVoid,
            RegistryPrepare,
            RegistryAuthorizeExport,
        ],
        Role::ClinicalSupport => &[
            PatientRead,
            PatientCreate,
            EncounterCreate,
            EncounterEditDraft,
            EncounterMarkReady,
        ],
        Role::FacilityAdministrator => &[UserManage, FacilityManage, BackupManage],
        Role::AuditorPrivacyReviewer => &[PatientRead, AuditRead],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiple_roles_are_combined() {
        let permissions =
            permissions_for_roles(&[Role::FacilityAdministrator, Role::VaccinatingProfessional]);
        assert!(permissions.contains(&Permission::UserManage));
        assert!(permissions.contains(&Permission::AdministrationConfirm));
    }

    #[test]
    fn facility_administrator_has_no_clinical_authority() {
        let permissions = permissions_for_roles(&[Role::FacilityAdministrator]);
        assert!(!permissions.contains(&Permission::AdministrationConfirm));
        assert!(!permissions.contains(&Permission::PatientRead));
    }
}
