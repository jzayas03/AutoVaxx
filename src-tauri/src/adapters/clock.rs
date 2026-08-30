use chrono::{Offset, Utc};
use chrono_tz::Tz;

use crate::domain::ClinicalTime;
use crate::error::AppError;
use crate::ports::Clock;

pub struct SystemClock {
    timezone_name: String,
    timezone: Tz,
}

impl SystemClock {
    pub fn puerto_rico() -> Self {
        Self {
            timezone_name: "America/Puerto_Rico".to_owned(),
            timezone: chrono_tz::America::Puerto_Rico,
        }
    }

    pub fn from_timezone(timezone_name: &str) -> Result<Self, AppError> {
        let timezone = timezone_name
            .parse::<Tz>()
            .map_err(|_| AppError::Validation)?;
        Ok(Self {
            timezone_name: timezone_name.to_owned(),
            timezone,
        })
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Result<ClinicalTime, AppError> {
        let utc = Utc::now();
        let local = utc.with_timezone(&self.timezone);
        Ok(ClinicalTime::from_parts(
            utc,
            local.naive_local(),
            self.timezone_name.clone(),
            local.offset().fix(),
        ))
    }
}

#[cfg(test)]
pub struct FixedClock(pub ClinicalTime);

#[cfg(test)]
impl Clock for FixedClock {
    fn now(&self) -> Result<ClinicalTime, AppError> {
        Ok(self.0.clone())
    }
}
