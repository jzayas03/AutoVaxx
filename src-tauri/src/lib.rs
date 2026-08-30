#![forbid(unsafe_code)]

#[cfg(all(feature = "production", feature = "synthetic-only"))]
compile_error!("production and synthetic-only features are mutually exclusive");

#[cfg(all(feature = "production", feature = "dev-auth"))]
compile_error!("DEV_ONLY authentication cannot be compiled into production");

#[cfg(all(feature = "production", not(feature = "sqlcipher")))]
compile_error!("production builds require the SQLCipher feature");

#[cfg(all(feature = "production", not(feature = "windows-secret-store")))]
compile_error!("production builds require the Windows SecretStore feature");

#[cfg(all(feature = "production", not(feature = "production-logging")))]
compile_error!("production builds require the PHI-safe production logging policy");

#[cfg(all(feature = "production", not(target_os = "windows")))]
compile_error!("the Phase 1 production target is Windows only");

#[cfg(all(feature = "real-phi", not(feature = "production")))]
compile_error!("real-PHI builds require the complete production feature set");

#[cfg(all(feature = "real-phi", not(feature = "approved-schema")))]
compile_error!("real-PHI builds require an approved schema");

#[cfg(all(feature = "real-phi", not(feature = "hardened-security-config")))]
compile_error!("real-PHI builds require hardened security configuration");

pub mod adapters;
pub mod application;
pub mod domain;
pub mod error;
pub mod ports;

use std::sync::Arc;

use application::{AppState, commands};
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| error.to_string())?;
            std::fs::create_dir_all(&data_dir)?;
            let database_path = data_dir.join("autovaxx-synthetic.sqlite");
            let state = AppState::initialize_synthetic(&database_path)?;
            app.manage(Arc::new(state));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::foundation_status,
            commands::login,
            commands::create_patient,
            commands::get_patient,
            commands::create_encounter,
            commands::transition_encounter,
        ])
        .run(tauri::generate_context!())
        .expect("AutoVaxx application runtime failed");
}
