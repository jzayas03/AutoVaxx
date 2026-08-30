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
    pub production_secret_store: bool,
    pub production_logging_policy: bool,
    pub approved_schema: bool,
    pub required_security_configuration: bool,
    pub real_phi_enabled: bool,
}

impl BuildCapabilities {
    pub const fn compiled() -> Self {
        Self {
            production: cfg!(feature = "production"),
            synthetic_only: cfg!(feature = "synthetic-only"),
            dev_auth: cfg!(feature = "dev-auth"),
            encrypted_database: cfg!(feature = "sqlcipher"),
            production_secret_store: cfg!(all(
                target_os = "windows",
                feature = "windows-secret-store"
            )),
            production_logging_policy: cfg!(feature = "production-logging"),
            approved_schema: cfg!(feature = "approved-schema"),
            required_security_configuration: cfg!(feature = "hardened-security-config"),
            real_phi_enabled: cfg!(feature = "real-phi"),
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
            && (!self.capabilities.production
                || !self.capabilities.real_phi_enabled
                || !self.capabilities.encrypted_database
                || !self.capabilities.production_secret_store
                || !self.capabilities.production_logging_policy
                || !self.capabilities.approved_schema
                || !self.capabilities.required_security_configuration
                || self.capabilities.synthetic_only
                || self.capabilities.dev_auth)
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
                production_secret_store: true,
                production_logging_policy: true,
                approved_schema: true,
                required_security_configuration: true,
                real_phi_enabled: true,
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
                production_secret_store: true,
                production_logging_policy: true,
                approved_schema: true,
                required_security_configuration: true,
                real_phi_enabled: true,
            },
        };
        assert!(matches!(config.validate(), Err(AppError::Configuration)));
    }

    #[test]
    fn current_build_accepts_only_synthetic_data() {
        let result = RuntimeConfig::synthetic_only().validate();
        if cfg!(feature = "synthetic-only") {
            assert!(result.is_ok());
        } else {
            assert!(matches!(result, Err(AppError::Configuration)));
        }
    }

    #[test]
    fn real_phi_requires_every_production_capability() {
        let complete = BuildCapabilities {
            production: true,
            synthetic_only: false,
            dev_auth: false,
            encrypted_database: true,
            production_secret_store: true,
            production_logging_policy: true,
            approved_schema: true,
            required_security_configuration: true,
            real_phi_enabled: true,
        };
        assert!(
            RuntimeConfig {
                data_mode: DataMode::RealPhi,
                capabilities: complete,
            }
            .validate()
            .is_ok()
        );

        for missing in 0..5 {
            let mut capabilities = complete;
            match missing {
                0 => capabilities.production_secret_store = false,
                1 => capabilities.production_logging_policy = false,
                2 => capabilities.approved_schema = false,
                3 => capabilities.required_security_configuration = false,
                4 => capabilities.real_phi_enabled = false,
                _ => unreachable!(),
            }
            assert!(matches!(
                RuntimeConfig {
                    data_mode: DataMode::RealPhi,
                    capabilities,
                }
                .validate(),
                Err(AppError::Configuration)
            ));
        }
    }
}
