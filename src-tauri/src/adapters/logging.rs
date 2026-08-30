use serde::Serialize;
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OperationalComponent {
    Application,
    Persistence,
    Backup,
    Authentication,
    Provider,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OperationalEventCode {
    Started,
    Completed,
    Failed,
    Unavailable,
    Rejected,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationalEvent {
    pub component: OperationalComponent,
    pub event_code: OperationalEventCode,
    pub correlation_id: Uuid,
    pub duration_ms: Option<u64>,
    pub software_version: &'static str,
}

impl OperationalEvent {
    pub fn safe_json(&self) -> Result<String, AppError> {
        Ok(serde_json::to_string(self)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operational_log_schema_has_no_field_for_phi() {
        let marker = "SYNTHETIC-PATIENT-MARKER-JANE-DOE-2000-01-01";
        let event = OperationalEvent {
            component: OperationalComponent::Persistence,
            event_code: OperationalEventCode::Completed,
            correlation_id: Uuid::new_v4(),
            duration_ms: Some(12),
            software_version: env!("CARGO_PKG_VERSION"),
        };
        let serialized = event.safe_json().unwrap();
        assert!(!serialized.contains(marker));
        assert!(!serialized.contains("patientName"));
        assert!(!serialized.contains("dateOfBirth"));
    }
}
