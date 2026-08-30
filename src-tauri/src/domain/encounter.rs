use serde::{Deserialize, Serialize};

use crate::error::AppError;

use super::{EncounterId, FacilityId, PatientId, Permission, UserId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EncounterState {
    Draft,
    ReadyToAdminister,
    AdministeredPendingDocumentation,
    Finalized,
    RegistryReady,
    Corrected,
    Voided,
}

impl EncounterState {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Draft => "DRAFT",
            Self::ReadyToAdminister => "READY_TO_ADMINISTER",
            Self::AdministeredPendingDocumentation => "ADMINISTERED_PENDING_DOCUMENTATION",
            Self::Finalized => "FINALIZED",
            Self::RegistryReady => "REGISTRY_READY",
            Self::Corrected => "CORRECTED",
            Self::Voided => "VOIDED",
        }
    }

    pub fn parse(value: &str) -> Result<Self, AppError> {
        match value {
            "DRAFT" => Ok(Self::Draft),
            "READY_TO_ADMINISTER" => Ok(Self::ReadyToAdminister),
            "ADMINISTERED_PENDING_DOCUMENTATION" => Ok(Self::AdministeredPendingDocumentation),
            "FINALIZED" => Ok(Self::Finalized),
            "REGISTRY_READY" => Ok(Self::RegistryReady),
            "CORRECTED" => Ok(Self::Corrected),
            "VOIDED" => Ok(Self::Voided),
            _ => Err(AppError::Validation),
        }
    }

    pub fn required_permission_for_transition(self, target: Self) -> Permission {
        use EncounterState::*;
        match (self, target) {
            (_, Voided) => Permission::EncounterVoid,
            (Draft, ReadyToAdminister) => Permission::EncounterMarkReady,
            (ReadyToAdminister, Draft) => Permission::EncounterEditDraft,
            (ReadyToAdminister, AdministeredPendingDocumentation) => {
                Permission::AdministrationConfirm
            }
            (AdministeredPendingDocumentation, Finalized) | (Corrected, Finalized) => {
                Permission::EncounterFinalize
            }
            (Finalized, RegistryReady) | (Corrected, RegistryReady) => Permission::RegistryPrepare,
            (Finalized, Corrected)
            | (RegistryReady, Corrected)
            | (AdministeredPendingDocumentation, Corrected) => Permission::EncounterCorrect,
            _ => Permission::EncounterEditDraft,
        }
    }

    pub fn transition_to(self, target: Self) -> Result<Self, AppError> {
        use EncounterState::*;
        let valid = matches!(
            (self, target),
            (Draft, ReadyToAdminister)
                | (Draft, Voided)
                | (ReadyToAdminister, Draft)
                | (ReadyToAdminister, AdministeredPendingDocumentation)
                | (ReadyToAdminister, Voided)
                | (AdministeredPendingDocumentation, Finalized)
                | (AdministeredPendingDocumentation, Corrected)
                | (AdministeredPendingDocumentation, Voided)
                | (Finalized, RegistryReady)
                | (Finalized, Corrected)
                | (Finalized, Voided)
                | (RegistryReady, Corrected)
                | (RegistryReady, Voided)
                | (Corrected, Finalized)
                | (Corrected, RegistryReady)
                | (Corrected, Voided)
        );
        valid.then_some(target).ok_or(AppError::InvalidTransition)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImmunizationEncounter {
    pub encounter_id: EncounterId,
    pub patient_id: PatientId,
    pub facility_id: FacilityId,
    pub responsible_professional_id: UserId,
    pub state: EncounterState,
    pub revision: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_state_transitions_are_explicit() {
        assert_eq!(
            EncounterState::ReadyToAdminister
                .transition_to(EncounterState::AdministeredPendingDocumentation)
                .unwrap(),
            EncounterState::AdministeredPendingDocumentation
        );
        assert_eq!(
            EncounterState::Finalized
                .transition_to(EncounterState::Corrected)
                .unwrap(),
            EncounterState::Corrected
        );
    }

    #[test]
    fn invalid_state_transition_is_rejected() {
        assert!(matches!(
            EncounterState::Draft.transition_to(EncounterState::Finalized),
            Err(AppError::InvalidTransition)
        ));
        assert!(
            EncounterState::Voided
                .transition_to(EncounterState::Draft)
                .is_err()
        );
    }
}
