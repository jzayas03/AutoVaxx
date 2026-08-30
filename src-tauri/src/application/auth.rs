use std::time::{Duration, Instant};

use argon2::password_hash::{
    PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng,
};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, Duration as ChronoDuration, Utc};

use crate::adapters::Database;
use crate::domain::{SessionContext, User, WorkstationId};
use crate::error::AppError;

pub const ARGON2_MEMORY_KIB: u32 = 65_536;
pub const ARGON2_ITERATIONS: u32 = 3;
pub const ARGON2_PARALLELISM: u32 = 1;

fn argon2id() -> Result<Argon2<'static>, AppError> {
    let params = Params::new(
        ARGON2_MEMORY_KIB,
        ARGON2_ITERATIONS,
        ARGON2_PARALLELISM,
        None,
    )
    .map_err(|_| AppError::Cryptography)?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

pub fn hash_password(password: &[u8]) -> Result<String, AppError> {
    if password.len() < 12 || password.len() > 1024 {
        return Err(AppError::Validation);
    }
    let salt = SaltString::generate(&mut OsRng);
    argon2id()?
        .hash_password(password, &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| AppError::Cryptography)
}

pub fn verify_password(password: &[u8], encoded_verifier: &str) -> Result<bool, AppError> {
    let parsed = PasswordHash::new(encoded_verifier).map_err(|_| AppError::Authentication)?;
    Ok(argon2id()?.verify_password(password, &parsed).is_ok())
}

pub fn benchmark_argon2id(password: &[u8]) -> Result<Duration, AppError> {
    let started = Instant::now();
    let _ = hash_password(password)?;
    Ok(started.elapsed())
}

pub struct AuthService<'a> {
    database: &'a Database,
}

impl<'a> AuthService<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn login(
        &self,
        username: &str,
        password: &[u8],
        workstation_id: WorkstationId,
        now: DateTime<Utc>,
    ) -> Result<(String, SessionContext), AppError> {
        let credential = self
            .database
            .credential_by_username(username)?
            .ok_or(AppError::Authentication)?;
        if !credential.user.active || credential.locked_until.is_some_and(|until| until > now) {
            return Err(AppError::Authentication);
        }
        if !verify_password(password, &credential.password_verifier)? {
            self.database.record_failed_login(
                credential.user.user_id,
                credential.facility_id,
                workstation_id,
                now,
            )?;
            return Err(AppError::Authentication);
        }
        let raw_token = new_session_token()?;
        let session =
            self.database
                .create_session(&credential.user, workstation_id, &raw_token, now)?;
        Ok((raw_token, session))
    }

    pub fn authenticate(
        &self,
        raw_token: &str,
        now: DateTime<Utc>,
    ) -> Result<SessionContext, AppError> {
        if raw_token.len() < 32 || raw_token.len() > 256 {
            return Err(AppError::Authentication);
        }
        self.database.session_by_token(raw_token, now)
    }

    pub fn require_recent_auth(
        session: &SessionContext,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        if now - session.recent_auth_at > ChronoDuration::minutes(5) {
            Err(AppError::Authentication)
        } else {
            Ok(())
        }
    }
}

fn new_session_token() -> Result<String, AppError> {
    let mut token = [0_u8; 32];
    getrandom::fill(&mut token).map_err(|_| AppError::Cryptography)?;
    Ok(URL_SAFE_NO_PAD.encode(token))
}

pub fn create_user_with_password(
    database: &Database,
    user: &User,
    facility_id: crate::domain::FacilityId,
    password: &[u8],
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let verifier = hash_password(password)?;
    database.create_user(user, facility_id, &verifier, now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifier_is_versioned_salted_and_not_recoverable() {
        let first = hash_password(b"synthetic-password-one").unwrap();
        let second = hash_password(b"synthetic-password-one").unwrap();
        assert!(first.starts_with("$argon2id$v=19$"));
        assert_ne!(first, second);
        assert!(!first.contains("synthetic-password-one"));
        assert!(verify_password(b"synthetic-password-one", &first).unwrap());
        assert!(!verify_password(b"different-password", &first).unwrap());
    }

    #[test]
    fn selected_argon2id_parameters_are_benchmarked_on_this_host() {
        let duration = benchmark_argon2id(b"synthetic-password-for-benchmark").unwrap();
        eprintln!(
            "argon2id m={}KiB,t={},p={} elapsed_ms={}",
            ARGON2_MEMORY_KIB,
            ARGON2_ITERATIONS,
            ARGON2_PARALLELISM,
            duration.as_millis()
        );
        assert!(duration.as_nanos() > 0);
    }
}
