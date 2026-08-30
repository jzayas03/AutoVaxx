use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! opaque_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

opaque_id!(UserId);
opaque_id!(FacilityId);
opaque_id!(PatientId);
opaque_id!(EncounterId);
opaque_id!(AuditEventId);
opaque_id!(SessionId);
opaque_id!(WorkstationId);
opaque_id!(VaccinationAdministrationId);
opaque_id!(ScreeningRevisionId);
opaque_id!(ConsentRevisionId);
opaque_id!(VisDeliveryId);
opaque_id!(ImmunizationRevisionId);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Facility {
    pub facility_id: FacilityId,
    pub name: String,
    pub timezone: String,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub user_id: UserId,
    pub username: String,
    pub display_name: String,
    pub active: bool,
    pub roles: Vec<Role>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaccinationAdministration {
    pub administration_id: VaccinationAdministrationId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreeningRevision {
    pub screening_revision_id: ScreeningRevisionId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentRevision {
    pub consent_revision_id: ConsentRevisionId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisDelivery {
    pub vis_delivery_id: VisDeliveryId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImmunizationRevision {
    pub immunization_revision_id: ImmunizationRevisionId,
}

use super::Role;
