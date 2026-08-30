use std::str::FromStr;
use std::sync::Arc;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::application::AppState;
use crate::application::auth::AuthService;
use crate::application::config::DataMode;
use crate::application::services::{EncounterService, PatientService};
use crate::domain::{
    EncounterId, EncounterState, ExternalIdentifier, FacilityId, ImmunizationEncounter, Patient,
    PatientAddress, PatientId, PatientName, UserId, WorkstationId,
};
use crate::error::{AppError, CommandError};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FoundationStatus {
    pub data_mode: DataMode,
    pub external_egress_required: bool,
    pub clinical_decision_support_enabled: bool,
    pub production_ready: bool,
}

#[tauri::command]
pub fn foundation_status() -> FoundationStatus {
    FoundationStatus {
        data_mode: DataMode::SyntheticOnly,
        external_egress_required: false,
        clinical_decision_support_enabled: false,
        production_ready: false,
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub workstation_id: WorkstationId,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub session_token: String,
    pub expires_at_utc: String,
}

#[tauri::command]
pub fn login(
    state: State<'_, Arc<AppState>>,
    request: LoginRequest,
) -> Result<LoginResponse, CommandError> {
    login_impl(&state, request).map_err(Into::into)
}

pub fn login_impl(state: &AppState, request: LoginRequest) -> Result<LoginResponse, AppError> {
    let now = state.clock.now()?.utc_instant;
    let (session_token, session) = AuthService::new(&state.database).login(
        &request.username,
        request.password.as_bytes(),
        request.workstation_id,
        now,
    )?;
    Ok(LoginResponse {
        session_token,
        expires_at_utc: session.expires_at.to_rfc3339(),
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePatientRequest {
    pub given_names: String,
    pub middle_names: Option<String>,
    pub first_surname: String,
    pub second_surname: Option<String>,
    pub suffix: Option<String>,
    pub preferred_name: Option<String>,
    pub date_of_birth: String,
    pub address: Option<PatientAddress>,
    pub external_identifiers: Vec<ExternalIdentifier>,
}

#[tauri::command]
pub fn create_patient(
    state: State<'_, Arc<AppState>>,
    session_token: String,
    request: CreatePatientRequest,
) -> Result<Patient, CommandError> {
    create_patient_impl(&state, &session_token, request).map_err(Into::into)
}

pub fn create_patient_impl(
    state: &AppState,
    session_token: &str,
    request: CreatePatientRequest,
) -> Result<Patient, AppError> {
    let now = state.clock.now()?.utc_instant;
    let actor = AuthService::new(&state.database).authenticate(session_token, now)?;
    let patient = Patient {
        patient_id: PatientId::new(),
        revision: 1,
        name: PatientName {
            given_names: request.given_names,
            middle_names: request.middle_names,
            first_surname: request.first_surname,
            second_surname: request.second_surname,
            suffix: request.suffix,
            preferred_name: request.preferred_name,
        },
        date_of_birth: NaiveDate::from_str(&request.date_of_birth)
            .map_err(|_| AppError::Validation)?,
        address: request.address,
        external_identifiers: request.external_identifiers,
        created_by: actor.user_id,
    };
    PatientService::new(&state.database).create(&actor, &patient)?;
    Ok(patient)
}

#[tauri::command]
pub fn get_patient(
    state: State<'_, Arc<AppState>>,
    session_token: String,
    patient_id: PatientId,
) -> Result<Patient, CommandError> {
    get_patient_impl(&state, &session_token, patient_id).map_err(Into::into)
}

pub fn get_patient_impl(
    state: &AppState,
    session_token: &str,
    patient_id: PatientId,
) -> Result<Patient, AppError> {
    let actor = AuthService::new(&state.database)
        .authenticate(session_token, state.clock.now()?.utc_instant)?;
    PatientService::new(&state.database).get(&actor, patient_id)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateEncounterRequest {
    pub patient_id: PatientId,
    pub facility_id: FacilityId,
    pub responsible_professional_id: UserId,
}

#[tauri::command]
pub fn create_encounter(
    state: State<'_, Arc<AppState>>,
    session_token: String,
    request: CreateEncounterRequest,
) -> Result<ImmunizationEncounter, CommandError> {
    create_encounter_impl(&state, &session_token, request).map_err(Into::into)
}

pub fn create_encounter_impl(
    state: &AppState,
    session_token: &str,
    request: CreateEncounterRequest,
) -> Result<ImmunizationEncounter, AppError> {
    let actor = AuthService::new(&state.database)
        .authenticate(session_token, state.clock.now()?.utc_instant)?;
    let encounter = ImmunizationEncounter {
        encounter_id: EncounterId::new(),
        patient_id: request.patient_id,
        facility_id: request.facility_id,
        responsible_professional_id: request.responsible_professional_id,
        state: EncounterState::Draft,
        revision: 1,
    };
    EncounterService::new(&state.database).create(&actor, &encounter)?;
    Ok(encounter)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransitionEncounterRequest {
    pub encounter_id: EncounterId,
    pub expected_revision: u64,
    pub target_state: EncounterState,
}

#[tauri::command]
pub fn transition_encounter(
    state: State<'_, Arc<AppState>>,
    session_token: String,
    request: TransitionEncounterRequest,
) -> Result<ImmunizationEncounter, CommandError> {
    transition_encounter_impl(&state, &session_token, request).map_err(Into::into)
}

pub fn transition_encounter_impl(
    state: &AppState,
    session_token: &str,
    request: TransitionEncounterRequest,
) -> Result<ImmunizationEncounter, AppError> {
    let now = state.clock.now()?.utc_instant;
    let actor = AuthService::new(&state.database).authenticate(session_token, now)?;
    EncounterService::new(&state.database).transition(
        &actor,
        request.encounter_id,
        request.expected_revision,
        request.target_state,
        now,
    )
}

pub fn parse_uuid(value: &str) -> Result<Uuid, AppError> {
    Uuid::from_str(value).map_err(|_| AppError::Validation)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{FixedOffset, TimeZone, Utc};

    use super::*;
    use crate::adapters::FixedClock;
    use crate::application::auth::create_user_with_password;
    use crate::domain::{ClinicalTime, Facility, Role, User};

    struct Fixture {
        state: AppState,
        facility_id: FacilityId,
        workstation_id: WorkstationId,
        professional: User,
        support: User,
        professional_token: String,
        support_token: String,
    }

    fn fixed_time() -> ClinicalTime {
        let utc = Utc.with_ymd_and_hms(2026, 8, 30, 16, 0, 0).unwrap();
        let offset = FixedOffset::west_opt(4 * 60 * 60).unwrap();
        ClinicalTime::from_parts(
            utc,
            utc.with_timezone(&offset).naive_local(),
            "America/Puerto_Rico".to_owned(),
            offset,
        )
    }

    fn fixture() -> Fixture {
        let time = fixed_time();
        let now = time.utc_instant;
        let state = AppState::synthetic_in_memory(Arc::new(FixedClock(time))).unwrap();
        let facility_id = FacilityId::new();
        let workstation_id = WorkstationId::new();
        let facility = Facility {
            facility_id,
            name: "Synthetic San Juan Pharmacy".to_owned(),
            timezone: "America/Puerto_Rico".to_owned(),
            active: true,
        };
        state
            .database
            .create_facility_and_workstation(&facility, workstation_id, "SYNTHETIC-WS", now)
            .unwrap();
        let professional = User {
            user_id: UserId::new(),
            username: "synthetic.professional".to_owned(),
            display_name: "Synthetic Vaccinating Professional".to_owned(),
            active: true,
            roles: vec![Role::VaccinatingProfessional],
        };
        let support = User {
            user_id: UserId::new(),
            username: "synthetic.support".to_owned(),
            display_name: "Synthetic Clinical Support".to_owned(),
            active: true,
            roles: vec![Role::ClinicalSupport],
        };
        create_user_with_password(
            &state.database,
            &professional,
            facility_id,
            b"synthetic-professional-password",
            now,
        )
        .unwrap();
        create_user_with_password(
            &state.database,
            &support,
            facility_id,
            b"synthetic-support-password",
            now,
        )
        .unwrap();
        let professional_token = login_impl(
            &state,
            LoginRequest {
                username: professional.username.clone(),
                password: "synthetic-professional-password".to_owned(),
                workstation_id,
            },
        )
        .unwrap()
        .session_token;
        let support_token = login_impl(
            &state,
            LoginRequest {
                username: support.username.clone(),
                password: "synthetic-support-password".to_owned(),
                workstation_id,
            },
        )
        .unwrap()
        .session_token;
        Fixture {
            state,
            facility_id,
            workstation_id,
            professional,
            support,
            professional_token,
            support_token,
        }
    }

    fn synthetic_patient_request(identifier_value: &str) -> CreatePatientRequest {
        CreatePatientRequest {
            given_names: "Marisol Synthetic".to_owned(),
            middle_names: Some("Prueba".to_owned()),
            first_surname: "Ejemplo".to_owned(),
            second_surname: Some("Ficticia".to_owned()),
            suffix: None,
            preferred_name: Some("Soli".to_owned()),
            date_of_birth: "2000-01-01".to_owned(),
            address: Some(PatientAddress {
                line1: "100 Calle Ficticia".to_owned(),
                line2: None,
                municipality: "San Juan".to_owned(),
                region: "PR".to_owned(),
                postal_code: "00900".to_owned(),
                country_code: "US".to_owned(),
            }),
            external_identifiers: vec![ExternalIdentifier {
                identifier_type: "SYNTHETIC_TEST_ID".to_owned(),
                assigning_authority: "AUTOVAXX_TEST".to_owned(),
                value: identifier_value.to_owned(),
            }],
        }
    }

    #[test]
    fn patient_creation_and_retrieval_use_the_authenticated_boundary() {
        let fixture = fixture();
        let created = create_patient_impl(
            &fixture.state,
            &fixture.support_token,
            synthetic_patient_request("SYN-0001"),
        )
        .unwrap();
        let retrieved = get_patient_impl(
            &fixture.state,
            &fixture.professional_token,
            created.patient_id,
        )
        .unwrap();
        assert_eq!(created, retrieved);
        assert_eq!(retrieved.name.first_surname, "Ejemplo");
        assert_eq!(retrieved.name.second_surname.as_deref(), Some("Ficticia"));
        assert_eq!(fixture.state.database.audit_event_count_value().unwrap(), 3);
    }

    #[test]
    fn encounter_creation_and_valid_transitions_are_persisted() {
        let fixture = fixture();
        let patient = create_patient_impl(
            &fixture.state,
            &fixture.support_token,
            synthetic_patient_request("SYN-0002"),
        )
        .unwrap();
        let encounter = create_encounter_impl(
            &fixture.state,
            &fixture.support_token,
            CreateEncounterRequest {
                patient_id: patient.patient_id,
                facility_id: fixture.facility_id,
                responsible_professional_id: fixture.professional.user_id,
            },
        )
        .unwrap();
        let ready = transition_encounter_impl(
            &fixture.state,
            &fixture.support_token,
            TransitionEncounterRequest {
                encounter_id: encounter.encounter_id,
                expected_revision: 1,
                target_state: EncounterState::ReadyToAdminister,
            },
        )
        .unwrap();
        let administered = transition_encounter_impl(
            &fixture.state,
            &fixture.professional_token,
            TransitionEncounterRequest {
                encounter_id: encounter.encounter_id,
                expected_revision: ready.revision,
                target_state: EncounterState::AdministeredPendingDocumentation,
            },
        )
        .unwrap();
        assert_eq!(
            administered.state,
            EncounterState::AdministeredPendingDocumentation
        );
        assert_eq!(administered.revision, 3);
    }

    #[test]
    fn forged_tauri_command_cannot_bypass_rust_authorization() {
        let fixture = fixture();
        let patient = create_patient_impl(
            &fixture.state,
            &fixture.support_token,
            synthetic_patient_request("SYN-0003"),
        )
        .unwrap();
        let encounter = create_encounter_impl(
            &fixture.state,
            &fixture.support_token,
            CreateEncounterRequest {
                patient_id: patient.patient_id,
                facility_id: fixture.facility_id,
                responsible_professional_id: fixture.professional.user_id,
            },
        )
        .unwrap();
        let ready = transition_encounter_impl(
            &fixture.state,
            &fixture.support_token,
            TransitionEncounterRequest {
                encounter_id: encounter.encounter_id,
                expected_revision: 1,
                target_state: EncounterState::ReadyToAdminister,
            },
        )
        .unwrap();
        let forged = transition_encounter_impl(
            &fixture.state,
            &fixture.support_token,
            TransitionEncounterRequest {
                encounter_id: encounter.encounter_id,
                expected_revision: ready.revision,
                target_state: EncounterState::AdministeredPendingDocumentation,
            },
        );
        assert!(matches!(forged, Err(AppError::Authorization)));
        assert_eq!(
            fixture
                .state
                .database
                .get_encounter_by_id(encounter.encounter_id)
                .unwrap()
                .state,
            EncounterState::ReadyToAdminister
        );
    }

    #[test]
    fn stale_revision_is_rejected_and_audited_as_a_failure() {
        let fixture = fixture();
        let patient = create_patient_impl(
            &fixture.state,
            &fixture.support_token,
            synthetic_patient_request("SYN-0004"),
        )
        .unwrap();
        let encounter = create_encounter_impl(
            &fixture.state,
            &fixture.support_token,
            CreateEncounterRequest {
                patient_id: patient.patient_id,
                facility_id: fixture.facility_id,
                responsible_professional_id: fixture.professional.user_id,
            },
        )
        .unwrap();
        transition_encounter_impl(
            &fixture.state,
            &fixture.support_token,
            TransitionEncounterRequest {
                encounter_id: encounter.encounter_id,
                expected_revision: 1,
                target_state: EncounterState::ReadyToAdminister,
            },
        )
        .unwrap();
        let audit_count = fixture.state.database.audit_event_count_value().unwrap();
        let stale = transition_encounter_impl(
            &fixture.state,
            &fixture.support_token,
            TransitionEncounterRequest {
                encounter_id: encounter.encounter_id,
                expected_revision: 1,
                target_state: EncounterState::Draft,
            },
        );
        assert!(matches!(stale, Err(AppError::StaleRevision)));
        assert_eq!(
            fixture.state.database.audit_event_count_value().unwrap(),
            audit_count + 1
        );
    }

    #[test]
    fn failed_patient_transaction_rolls_back_record_and_audit() {
        let fixture = fixture();
        create_patient_impl(
            &fixture.state,
            &fixture.support_token,
            synthetic_patient_request("SYN-DUPLICATE"),
        )
        .unwrap();
        let audit_count = fixture.state.database.audit_event_count_value().unwrap();
        let patient_count = fixture.state.database.patient_count().unwrap();
        let duplicate = create_patient_impl(
            &fixture.state,
            &fixture.support_token,
            synthetic_patient_request("SYN-DUPLICATE"),
        );
        assert!(matches!(duplicate, Err(AppError::Persistence(_))));
        assert_eq!(
            fixture.state.database.patient_count().unwrap(),
            patient_count
        );
        assert_eq!(
            fixture.state.database.audit_event_count_value().unwrap(),
            audit_count + 1
        );
    }

    #[test]
    fn audit_rows_are_immutable_even_through_direct_sql() {
        let fixture = fixture();
        create_patient_impl(
            &fixture.state,
            &fixture.support_token,
            synthetic_patient_request("SYN-0005"),
        )
        .unwrap();
        assert!(
            fixture
                .state
                .database
                .execute_test_sql("UPDATE audit_events SET outcome = 'ALTERED'")
                .is_err()
        );
        assert!(
            fixture
                .state
                .database
                .execute_test_sql("DELETE FROM audit_events")
                .is_err()
        );
        assert_eq!(fixture.state.database.audit_event_count_value().unwrap(), 3);
    }

    #[test]
    fn role_fixture_itself_has_no_accidental_admin_overlap() {
        let fixture = fixture();
        assert_eq!(fixture.support.roles, vec![Role::ClinicalSupport]);
        assert_ne!(fixture.workstation_id.0, Uuid::nil());
    }

    #[test]
    fn facility_administrator_cannot_read_or_mutate_clinical_records_without_a_clinical_role() {
        let fixture = fixture();
        let now = fixed_time().utc_instant;
        let administrator = User {
            user_id: UserId::new(),
            username: "synthetic.administrator".to_owned(),
            display_name: "Synthetic Facility Administrator".to_owned(),
            active: true,
            roles: vec![Role::FacilityAdministrator],
        };
        create_user_with_password(
            &fixture.state.database,
            &administrator,
            fixture.facility_id,
            b"synthetic-administrator-password",
            now,
        )
        .unwrap();
        let token = login_impl(
            &fixture.state,
            LoginRequest {
                username: administrator.username,
                password: "synthetic-administrator-password".to_owned(),
                workstation_id: fixture.workstation_id,
            },
        )
        .unwrap()
        .session_token;
        let patient = create_patient_impl(
            &fixture.state,
            &fixture.support_token,
            synthetic_patient_request("SYN-ADMIN-DENIAL"),
        )
        .unwrap();
        assert!(matches!(
            get_patient_impl(&fixture.state, &token, patient.patient_id),
            Err(AppError::Authorization)
        ));
        assert!(matches!(
            create_patient_impl(
                &fixture.state,
                &token,
                synthetic_patient_request("SYN-ADMIN-CREATE-DENIAL")
            ),
            Err(AppError::Authorization)
        ));
    }

    #[test]
    fn five_failed_password_attempts_lock_the_named_account() {
        let fixture = fixture();
        for _ in 0..5 {
            assert!(matches!(
                login_impl(
                    &fixture.state,
                    LoginRequest {
                        username: fixture.support.username.clone(),
                        password: "incorrect-synthetic-password".to_owned(),
                        workstation_id: fixture.workstation_id,
                    }
                ),
                Err(AppError::Authentication)
            ));
        }
        assert!(matches!(
            login_impl(
                &fixture.state,
                LoginRequest {
                    username: fixture.support.username,
                    password: "synthetic-support-password".to_owned(),
                    workstation_id: fixture.workstation_id,
                }
            ),
            Err(AppError::Authentication)
        ));
    }
}
