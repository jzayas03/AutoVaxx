use crate::domain::ClinicalTime;
use crate::error::AppError;

pub trait Clock: Send + Sync {
    fn now(&self) -> Result<ClinicalTime, AppError>;
}
