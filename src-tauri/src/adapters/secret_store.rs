use std::collections::HashMap;
use std::sync::Mutex;

use zeroize::Zeroizing;

use crate::error::AppError;
use crate::ports::SecretStore;

const WINDOWS_CREDENTIAL_SERVICE: &str = "com.cuadradozayas.autovaxx.database-key";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretStoreFault {
    Unavailable,
    AccessDenied,
    Corrupted,
    ProtectFailed,
    UnprotectFailed,
}

#[derive(Default)]
pub struct FakeSecretStore {
    secrets: Mutex<HashMap<String, Zeroizing<Vec<u8>>>>,
    fault: Mutex<Option<SecretStoreFault>>,
}

impl FakeSecretStore {
    pub fn new() -> Self {
        Self {
            secrets: Mutex::new(HashMap::new()),
            fault: Mutex::new(None),
        }
    }

    pub fn set_available(&self, available: bool) {
        self.inject_fault((!available).then_some(SecretStoreFault::Unavailable));
    }

    pub fn inject_fault(&self, fault: Option<SecretStoreFault>) {
        *self.fault.lock().expect("fake secret-store fault lock") = fault;
    }

    pub fn replace_for_test(&self, key_reference: &str, secret: &[u8]) {
        self.secrets
            .lock()
            .expect("fake secret-store secrets lock")
            .insert(key_reference.to_owned(), Zeroizing::new(secret.to_vec()));
    }

    fn fault(&self) -> Result<Option<SecretStoreFault>, AppError> {
        self.fault
            .lock()
            .map(|fault| *fault)
            .map_err(|_| AppError::SecretStoreUnavailable)
    }
}

impl SecretStore for FakeSecretStore {
    fn store(&self, key_reference: &str, secret: &[u8]) -> Result<(), AppError> {
        match self.fault()? {
            Some(SecretStoreFault::Unavailable) => return Err(AppError::SecretStoreUnavailable),
            Some(SecretStoreFault::AccessDenied) => return Err(AppError::SecretAccessDenied),
            Some(SecretStoreFault::ProtectFailed) => return Err(AppError::SecretProtectFailed),
            Some(SecretStoreFault::Corrupted | SecretStoreFault::UnprotectFailed) | None => {}
        }
        let mut secrets = self
            .secrets
            .lock()
            .map_err(|_| AppError::SecretStoreUnavailable)?;
        if secrets.contains_key(key_reference) {
            return Err(AppError::SecretAlreadyExists);
        }
        secrets.insert(key_reference.to_owned(), Zeroizing::new(secret.to_vec()));
        Ok(())
    }

    fn load(&self, key_reference: &str) -> Result<Vec<u8>, AppError> {
        match self.fault()? {
            Some(SecretStoreFault::Unavailable) => return Err(AppError::SecretStoreUnavailable),
            Some(SecretStoreFault::AccessDenied) => return Err(AppError::SecretAccessDenied),
            Some(SecretStoreFault::Corrupted) => return Err(AppError::SecretCorrupted),
            Some(SecretStoreFault::UnprotectFailed) => {
                return Err(AppError::SecretUnprotectFailed);
            }
            Some(SecretStoreFault::ProtectFailed) | None => {}
        }
        self.secrets
            .lock()
            .map_err(|_| AppError::SecretStoreUnavailable)?
            .get(key_reference)
            .map(|secret| secret.to_vec())
            .ok_or(AppError::SecretNotFound)
    }

    fn delete(&self, key_reference: &str) -> Result<(), AppError> {
        match self.fault()? {
            Some(SecretStoreFault::Unavailable) => return Err(AppError::SecretStoreUnavailable),
            Some(SecretStoreFault::AccessDenied) => return Err(AppError::SecretAccessDenied),
            Some(SecretStoreFault::ProtectFailed) => return Err(AppError::SecretProtectFailed),
            Some(SecretStoreFault::Corrupted | SecretStoreFault::UnprotectFailed) | None => {}
        }
        let removed = self
            .secrets
            .lock()
            .map_err(|_| AppError::SecretStoreUnavailable)?
            .remove(key_reference);
        removed.map(|_| ()).ok_or(AppError::SecretNotFound)
    }
}

pub struct WindowsSecretStore {
    service_name: String,
    #[cfg(windows)]
    store: Option<std::sync::Arc<windows_native_keyring_store::Store>>,
}

impl WindowsSecretStore {
    pub fn new() -> Self {
        Self {
            service_name: WINDOWS_CREDENTIAL_SERVICE.to_owned(),
            #[cfg(windows)]
            store: windows_native_keyring_store::Store::new().ok(),
        }
    }

    #[cfg(windows)]
    fn entry(&self, key_reference: &str) -> Result<keyring_core::Entry, AppError> {
        use keyring_core::api::CredentialStoreApi;

        validate_key_reference(key_reference)?;
        let target = format!("AutoVaxx/{key_reference}");
        let modifiers = HashMap::from([("target", target.as_str()), ("persistence", "Local")]);
        self.store
            .as_ref()
            .ok_or(AppError::SecretStoreUnavailable)?
            .build(&self.service_name, key_reference, Some(&modifiers))
            .map_err(map_keyring_read_error)
    }
}

impl Default for WindowsSecretStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for WindowsSecretStore {
    fn store(&self, key_reference: &str, secret: &[u8]) -> Result<(), AppError> {
        #[cfg(windows)]
        {
            let entry = self.entry(key_reference)?;
            match entry.get_secret() {
                Ok(existing) => {
                    drop(Zeroizing::new(existing));
                    return Err(AppError::SecretAlreadyExists);
                }
                Err(keyring_core::Error::NoEntry) => {}
                Err(error) => return Err(map_keyring_read_error(error)),
            }
            return entry.set_secret(secret).map_err(map_keyring_protect_error);
        }
        #[cfg(not(windows))]
        {
            let _ = (key_reference, secret, &self.service_name);
            Err(AppError::SecretStoreUnavailable)
        }
    }

    fn load(&self, key_reference: &str) -> Result<Vec<u8>, AppError> {
        #[cfg(windows)]
        {
            return self
                .entry(key_reference)?
                .get_secret()
                .map_err(map_keyring_read_error);
        }
        #[cfg(not(windows))]
        {
            let _ = (key_reference, &self.service_name);
            Err(AppError::SecretStoreUnavailable)
        }
    }

    fn delete(&self, key_reference: &str) -> Result<(), AppError> {
        #[cfg(windows)]
        {
            return self
                .entry(key_reference)?
                .delete_credential()
                .map_err(map_keyring_protect_error);
        }
        #[cfg(not(windows))]
        {
            let _ = (key_reference, &self.service_name);
            Err(AppError::SecretStoreUnavailable)
        }
    }
}

pub struct MacOsSecretStore;

impl SecretStore for MacOsSecretStore {
    fn store(&self, _key_reference: &str, _secret: &[u8]) -> Result<(), AppError> {
        Err(AppError::SecretStoreUnavailable)
    }

    fn load(&self, _key_reference: &str) -> Result<Vec<u8>, AppError> {
        Err(AppError::SecretStoreUnavailable)
    }

    fn delete(&self, _key_reference: &str) -> Result<(), AppError> {
        Err(AppError::SecretStoreUnavailable)
    }
}

#[cfg(any(windows, test))]
fn validate_key_reference(key_reference: &str) -> Result<(), AppError> {
    let valid = !key_reference.is_empty()
        && key_reference.len() <= 128
        && key_reference
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
    valid.then_some(()).ok_or(AppError::Validation)
}

#[cfg(windows)]
fn map_keyring_read_error(error: keyring_core::Error) -> AppError {
    match error {
        keyring_core::Error::NoEntry => AppError::SecretNotFound,
        keyring_core::Error::NoStorageAccess(_) => AppError::SecretAccessDenied,
        keyring_core::Error::BadDataFormat(_, _)
        | keyring_core::Error::BadStoreFormat(_)
        | keyring_core::Error::BadEncoding(_) => AppError::SecretCorrupted,
        keyring_core::Error::NoDefaultStore | keyring_core::Error::NotSupportedByStore(_) => {
            AppError::SecretStoreUnavailable
        }
        keyring_core::Error::Invalid(_, _) | keyring_core::Error::TooLong(_, _) => {
            AppError::Validation
        }
        keyring_core::Error::Ambiguous(_) | keyring_core::Error::PlatformFailure(_) => {
            AppError::SecretUnprotectFailed
        }
        _ => AppError::SecretUnprotectFailed,
    }
}

#[cfg(windows)]
fn map_keyring_protect_error(error: keyring_core::Error) -> AppError {
    match error {
        keyring_core::Error::NoEntry => AppError::SecretNotFound,
        keyring_core::Error::NoStorageAccess(_) => AppError::SecretAccessDenied,
        keyring_core::Error::NoDefaultStore | keyring_core::Error::NotSupportedByStore(_) => {
            AppError::SecretStoreUnavailable
        }
        keyring_core::Error::Invalid(_, _) | keyring_core::Error::TooLong(_, _) => {
            AppError::Validation
        }
        _ => AppError::SecretProtectFailed,
    }
}

pub fn generate_database_key() -> Result<Zeroizing<Vec<u8>>, AppError> {
    let mut key = Zeroizing::new(vec![0_u8; 32]);
    getrandom::fill(&mut key).map_err(|_| AppError::Cryptography)?;
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_database_key_round_trips_through_secret_store() {
        let store = FakeSecretStore::new();
        let key = generate_database_key().unwrap();
        assert_eq!(key.len(), 32);
        store.store("synthetic-db", &key).unwrap();
        assert_eq!(store.load("synthetic-db").unwrap(), *key);
    }

    #[test]
    fn unavailable_secret_store_fails_closed() {
        let store = FakeSecretStore::new();
        store.set_available(false);
        assert!(matches!(
            store.load("synthetic-db"),
            Err(AppError::SecretStoreUnavailable)
        ));
    }

    #[test]
    fn fake_store_refuses_silent_key_replacement() {
        let store = FakeSecretStore::new();
        store.store("synthetic-db", &[1; 32]).unwrap();
        assert!(matches!(
            store.store("synthetic-db", &[2; 32]),
            Err(AppError::SecretAlreadyExists)
        ));
        assert_eq!(store.load("synthetic-db").unwrap(), vec![1; 32]);
    }

    #[test]
    fn fake_store_exposes_typed_failure_seams() {
        let store = FakeSecretStore::new();
        store.inject_fault(Some(SecretStoreFault::AccessDenied));
        assert!(matches!(
            store.load("synthetic-db"),
            Err(AppError::SecretAccessDenied)
        ));
        store.inject_fault(Some(SecretStoreFault::Corrupted));
        assert!(matches!(
            store.load("synthetic-db"),
            Err(AppError::SecretCorrupted)
        ));
        store.inject_fault(Some(SecretStoreFault::UnprotectFailed));
        assert!(matches!(
            store.load("synthetic-db"),
            Err(AppError::SecretUnprotectFailed)
        ));
        store.inject_fault(Some(SecretStoreFault::ProtectFailed));
        assert!(matches!(
            store.store("synthetic-db", &[1; 32]),
            Err(AppError::SecretProtectFailed)
        ));
    }

    #[test]
    fn key_references_are_opaque_and_bounded() {
        assert!(validate_key_reference("autovaxx-db_01.test").is_ok());
        assert!(validate_key_reference("patient/name").is_err());
        assert!(validate_key_reference("").is_err());
    }

    #[cfg(windows)]
    #[test]
    fn windows_credential_store_protects_recovers_and_deletes_synthetic_key() {
        let store = WindowsSecretStore::new();
        let key_reference = format!("autovaxx-ci-{}", uuid::Uuid::new_v4());
        let key = generate_database_key().unwrap();
        store.store(&key_reference, &key).unwrap();
        assert_eq!(store.load(&key_reference).unwrap(), *key);
        assert!(matches!(
            store.store(&key_reference, &[9; 32]),
            Err(AppError::SecretAlreadyExists)
        ));
        store.delete(&key_reference).unwrap();
        assert!(matches!(
            store.load(&key_reference),
            Err(AppError::SecretNotFound)
        ));
    }
}
