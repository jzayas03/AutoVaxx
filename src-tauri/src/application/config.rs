use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DataMode {
    SyntheticOnly,
    RealPhi,
}

#[derive(Debug, Clone, Copy)]
pub struct BuildCapabilities {
    pub production: bool,
    pub synthetic_only: bool,
    pub dev_auth: bool,
    pub encrypted_database: bool,
}

impl BuildCapabilities {
    pub const fn compiled() -> Self {
        Self {
            production: cfg!(feature = "production"),
            synthetic_only: cfg!(feature = "synthetic-only"),
            dev_auth: cfg!(feature = "dev-auth"),
            encrypted_database: cfg!(feature = "sqlcipher"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RuntimeConfig {
    pub data_mode: DataMode,
    pub capabilities: BuildCapabilities,
}

impl RuntimeConfig {
    pub const fn synthetic_only() -> Self {
        Self {
            data_mode: DataMode::SyntheticOnly,
            capabilities: BuildCapabilities::compiled(),
        }
    }

    pub fn validate(self) -> Result<(), AppError> {
        if self.capabilities.production
            && (self.capabilities.synthetic_only || self.capabilities.dev_auth)
        {
            return Err(AppError::Configuration);
        }
        if self.capabilities.dev_auth && self.data_mode != DataMode::SyntheticOnly {
            return Err(AppError::Configuration);
        }
        if self.data_mode == DataMode::RealPhi
            && (!self.capabilities.production || !self.capabilities.encrypted_database)
        {
            return Err(AppError::Configuration);
        }
        if self.data_mode == DataMode::SyntheticOnly && !self.capabilities.synthetic_only {
            return Err(AppError::Configuration);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_rejects_dev_authentication() {
        let config = RuntimeConfig {
            data_mode: DataMode::RealPhi,
            capabilities: BuildCapabilities {
                production: true,
                synthetic_only: false,
                dev_auth: true,
                encrypted_database: true,
            },
        };
        assert!(matches!(config.validate(), Err(AppError::Configuration)));
    }

    #[test]
    fn real_phi_rejects_plaintext_database() {
        let config = RuntimeConfig {
            data_mode: DataMode::RealPhi,
            capabilities: BuildCapabilities {
                production: true,
                synthetic_only: false,
                dev_auth: false,
                encrypted_database: false,
            },
        };
        assert!(matches!(config.validate(), Err(AppError::Configuration)));
    }

    #[test]
    fn current_build_accepts_only_synthetic_data() {
        assert!(RuntimeConfig::synthetic_only().validate().is_ok());
    }
}
