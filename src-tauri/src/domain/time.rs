use chrono::{DateTime, FixedOffset, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClinicalTime {
    pub utc_instant: DateTime<Utc>,
    pub local_datetime: NaiveDateTime,
    pub timezone: String,
    pub utc_offset_minutes: i32,
}

impl ClinicalTime {
    pub fn from_parts(
        utc_instant: DateTime<Utc>,
        local_datetime: NaiveDateTime,
        timezone: String,
        offset: FixedOffset,
    ) -> Self {
        Self {
            utc_instant,
            local_datetime,
            timezone,
            utc_offset_minutes: offset.local_minus_utc() / 60,
        }
    }
}
