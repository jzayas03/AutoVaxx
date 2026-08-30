use crate::domain::{EncounterId, ImmunizationEncounter, Patient, PatientId, SessionContext};
use crate::error::AppError;

pub trait PatientRepository: Send + Sync {
    fn create_patient(&self, actor: &SessionContext, patient: &Patient) -> Result<(), AppError>;
    fn get_patient(&self, patient_id: PatientId) -> Result<Patient, AppError>;
}

pub trait EncounterRepository: Send + Sync {
    fn create_encounter(
        &self,
        actor: &SessionContext,
        encounter: &ImmunizationEncounter,
    ) -> Result<(), AppError>;
    fn get_encounter(&self, encounter_id: EncounterId) -> Result<ImmunizationEncounter, AppError>;
}

pub trait AuditRepository: Send + Sync {
    fn audit_event_count(&self) -> Result<u64, AppError>;
}
