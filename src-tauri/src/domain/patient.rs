use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::{PatientId, UserId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatientName {
    pub given_names: String,
    pub middle_names: Option<String>,
    pub first_surname: String,
    pub second_surname: Option<String>,
    pub suffix: Option<String>,
    pub preferred_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatientAddress {
    pub line1: String,
    pub line2: Option<String>,
    pub municipality: String,
    pub region: String,
    pub postal_code: String,
    pub country_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalIdentifier {
    pub identifier_type: String,
    pub assigning_authority: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Patient {
    pub patient_id: PatientId,
    pub revision: u64,
    pub name: PatientName,
    pub date_of_birth: NaiveDate,
    pub address: Option<PatientAddress>,
    pub external_identifiers: Vec<ExternalIdentifier>,
    pub created_by: UserId,
}

impl Patient {
    pub fn validate(&self) -> bool {
        !self.name.given_names.trim().is_empty()
            && !self.name.first_surname.trim().is_empty()
            && self.external_identifiers.iter().all(|identifier| {
                !identifier.identifier_type.trim().is_empty()
                    && !identifier.assigning_authority.trim().is_empty()
                    && !identifier.value.trim().is_empty()
            })
    }
}
