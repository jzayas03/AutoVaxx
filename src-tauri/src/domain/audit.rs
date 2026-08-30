use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{AuditEventId, FacilityId, SessionId, UserId, WorkstationId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEvent {
    pub audit_event_id: AuditEventId,
    pub occurred_at: DateTime<Utc>,
    pub actor_id: Option<UserId>,
    pub session_id: Option<SessionId>,
    pub workstation_id: WorkstationId,
    pub facility_id: FacilityId,
    pub action: String,
    pub entity_type: String,
    pub entity_id: String,
    pub entity_revision: Option<u64>,
    pub outcome: String,
    pub correlation_id: String,
    pub software_version: String,
    pub schema_version: u32,
    pub metadata_json: String,
    pub previous_hash: Option<String>,
    pub event_hash: String,
}
