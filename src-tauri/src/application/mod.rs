pub mod auth;
pub mod commands;
pub mod config;
pub mod services;

use std::path::Path;
use std::sync::Arc;

use crate::adapters::{Database, SystemClock};
use crate::error::AppError;
use crate::ports::Clock;

pub struct AppState {
    pub database: Arc<Database>,
    pub clock: Arc<dyn Clock>,
}

impl AppState {
    pub fn initialize_synthetic(path: &Path) -> Result<Self, AppError> {
        config::RuntimeConfig::synthetic_only().validate()?;
        Ok(Self {
            database: Arc::new(Database::open_synthetic(path)?),
            clock: Arc::new(SystemClock::puerto_rico()),
        })
    }

    #[cfg(test)]
    pub fn synthetic_in_memory(clock: Arc<dyn Clock>) -> Result<Self, AppError> {
        Ok(Self {
            database: Arc::new(Database::open_synthetic_in_memory()?),
            clock,
        })
    }
}
