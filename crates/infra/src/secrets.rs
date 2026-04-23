use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use relxen_app::{AppError, AppResult, SecretStore};
use relxen_domain::{LiveCredentialId, LiveCredentialSecret};

const SERVICE_NAME: &str = "relxen.live.credentials";

#[derive(Debug, Default)]
pub struct OsSecretStore;

#[async_trait]
impl SecretStore for OsSecretStore {
    async fn store(&self, id: &LiveCredentialId, secret: &LiveCredentialSecret) -> AppResult<()> {
        let entry = keyring::Entry::new(SERVICE_NAME, id.as_str())
            .map_err(|error| AppError::SecureStoreUnavailable(error.to_string()))?;
        let encoded = serde_json::to_string(secret)
            .map_err(|error| AppError::SecureStoreUnavailable(error.to_string()))?;
        entry
            .set_password(&encoded)
            .map_err(|error| AppError::SecureStoreUnavailable(error.to_string()))
    }

    async fn read(&self, id: &LiveCredentialId) -> AppResult<LiveCredentialSecret> {
        let entry = keyring::Entry::new(SERVICE_NAME, id.as_str())
            .map_err(|error| AppError::SecureStoreUnavailable(error.to_string()))?;
        let encoded = entry
            .get_password()
            .map_err(|error| AppError::SecureStoreUnavailable(error.to_string()))?;
        serde_json::from_str(&encoded)
            .map_err(|error| AppError::SecureStoreUnavailable(error.to_string()))
    }

    async fn delete(&self, id: &LiveCredentialId) -> AppResult<()> {
        let entry = keyring::Entry::new(SERVICE_NAME, id.as_str())
            .map_err(|error| AppError::SecureStoreUnavailable(error.to_string()))?;
        match entry.delete_password() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(AppError::SecureStoreUnavailable(error.to_string())),
        }
    }

    async fn ensure_available(&self) -> AppResult<()> {
        let probe = LiveCredentialId::new("__relxen_probe__");
        let secret = LiveCredentialSecret {
            api_key: "probe".to_string(),
            api_secret: "probe".to_string(),
        };
        self.store(&probe, &secret).await?;
        let _ = self.delete(&probe).await;
        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
pub struct MemorySecretStore {
    secrets: Arc<Mutex<BTreeMap<String, LiveCredentialSecret>>>,
    available: bool,
}

impl MemorySecretStore {
    pub fn new() -> Self {
        Self {
            secrets: Arc::new(Mutex::new(BTreeMap::new())),
            available: true,
        }
    }

    pub fn unavailable() -> Self {
        Self {
            secrets: Arc::new(Mutex::new(BTreeMap::new())),
            available: false,
        }
    }
}

#[async_trait]
impl SecretStore for MemorySecretStore {
    async fn store(&self, id: &LiveCredentialId, secret: &LiveCredentialSecret) -> AppResult<()> {
        self.ensure_available().await?;
        self.secrets
            .lock()
            .await
            .insert(id.as_str().to_string(), secret.clone());
        Ok(())
    }

    async fn read(&self, id: &LiveCredentialId) -> AppResult<LiveCredentialSecret> {
        self.ensure_available().await?;
        self.secrets
            .lock()
            .await
            .get(id.as_str())
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("secret not found for credential {id}")))
    }

    async fn delete(&self, id: &LiveCredentialId) -> AppResult<()> {
        self.ensure_available().await?;
        self.secrets.lock().await.remove(id.as_str());
        Ok(())
    }

    async fn ensure_available(&self) -> AppResult<()> {
        if self.available {
            Ok(())
        } else {
            Err(AppError::SecureStoreUnavailable(
                "memory secure store configured unavailable".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MemorySecretStore, SecretStore};
    use relxen_domain::{LiveCredentialId, LiveCredentialSecret};

    #[tokio::test]
    async fn memory_secret_store_saves_reads_and_deletes() {
        let store = MemorySecretStore::new();
        let id = LiveCredentialId::new("cred-1");
        let secret = LiveCredentialSecret {
            api_key: "api-key".to_string(),
            api_secret: "api-secret".to_string(),
        };

        store.store(&id, &secret).await.unwrap();
        assert_eq!(store.read(&id).await.unwrap(), secret);
        store.delete(&id).await.unwrap();
        assert!(store.read(&id).await.is_err());
    }

    #[tokio::test]
    async fn unavailable_secret_store_returns_typed_failure() {
        let store = MemorySecretStore::unavailable();
        let id = LiveCredentialId::new("cred-1");
        let secret = LiveCredentialSecret {
            api_key: "api-key".to_string(),
            api_secret: "api-secret".to_string(),
        };

        let error = store.store(&id, &secret).await.unwrap_err();
        assert!(matches!(
            error,
            relxen_app::AppError::SecureStoreUnavailable(_)
        ));
    }
}
