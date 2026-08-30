use std::collections::HashMap;
use std::sync::Mutex;

use zeroize::Zeroizing;

use crate::error::AppError;
use crate::ports::SecretStore;

#[derive(Default)]
pub struct FakeSecretStore {
    secrets: Mutex<HashMap<String, Zeroizing<Vec<u8>>>>,
    available: Mutex<bool>,
}

impl FakeSecretStore {
    pub fn new() -> Self {
        Self {
            secrets: Mutex::new(HashMap::new()),
            available: Mutex::new(true),
        }
    }

    pub fn set_available(&self, available: bool) {
        *self
            .available
            .lock()
            .expect("fake secret-store availability lock") = available;
    }

    fn ensure_available(&self) -> Result<(), AppError> {
        if *self
            .available
            .lock()
            .map_err(|_| AppError::SecretStoreUnavailable)?
        {
            Ok(())
        } else {
            Err(AppError::SecretStoreUnavailable)
        }
    }
}

impl SecretStore for FakeSecretStore {
    fn store(&self, key_reference: &str, secret: &[u8]) -> Result<(), AppError> {
        self.ensure_available()?;
        self.secrets
            .lock()
            .map_err(|_| AppError::SecretStoreUnavailable)?
            .insert(key_reference.to_owned(), Zeroizing::new(secret.to_vec()));
        Ok(())
    }

    fn load(&self, key_reference: &str) -> Result<Vec<u8>, AppError> {
        self.ensure_available()?;
        self.secrets
            .lock()
            .map_err(|_| AppError::SecretStoreUnavailable)?
            .get(key_reference)
            .map(|secret| secret.to_vec())
            .ok_or(AppError::SecretStoreUnavailable)
    }

    fn delete(&self, key_reference: &str) -> Result<(), AppError> {
        self.ensure_available()?;
        self.secrets
            .lock()
            .map_err(|_| AppError::SecretStoreUnavailable)?
            .remove(key_reference);
        Ok(())
    }
}

pub struct WindowsSecretStore;

impl SecretStore for WindowsSecretStore {
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
}
