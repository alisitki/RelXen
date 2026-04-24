use std::env;
use std::path::{Path, PathBuf};

use anyhow::Context;
use relxen_app::{EnvCredentialConfig, EnvCredentialPair};

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_addr: String,
    pub database_url: String,
    pub frontend_dist: PathBuf,
    pub log_level: String,
    pub auto_start: bool,
    pub enable_mainnet_canary_execution: bool,
    pub enable_testnet_drill_helpers: bool,
    pub env_credentials: EnvCredentialConfig,
}

impl ServerConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();
        let bind_addr = env::var("RELXEN_BIND").unwrap_or_else(|_| "[::]:3000".to_string());
        let database_url = env::var("RELXEN_DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://var/relxen.sqlite3".to_string());
        let frontend_dist =
            env::var("RELXEN_FRONTEND_DIST").unwrap_or_else(|_| "web/dist".to_string());
        let log_level =
            env::var("RELXEN_LOG_LEVEL").unwrap_or_else(|_| "info,relxen=debug".to_string());
        let auto_start = env::var("RELXEN_AUTO_START")
            .ok()
            .map(|value| value.eq_ignore_ascii_case("true"))
            .unwrap_or(true);
        let enable_mainnet_canary_execution = env::var("RELXEN_ENABLE_MAINNET_CANARY_EXECUTION")
            .ok()
            .map(|value| value.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let enable_testnet_drill_helpers = env::var("RELXEN_ENABLE_TESTNET_DRILL_HELPERS")
            .ok()
            .map(|value| value.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let env_credentials = load_env_credentials();

        ensure_database_parent(&database_url)?;

        Ok(Self {
            bind_addr,
            database_url,
            frontend_dist: PathBuf::from(frontend_dist),
            log_level,
            auto_start,
            enable_mainnet_canary_execution,
            enable_testnet_drill_helpers,
            env_credentials,
        })
    }
}

fn load_env_credentials() -> EnvCredentialConfig {
    let source = env::var("RELXEN_CREDENTIAL_SOURCE").ok();
    let compatibility_alias = env::var("RELXEN_ENABLE_ENV_CREDENTIALS").ok();
    let enabled = credential_source_enabled(source.as_deref(), compatibility_alias.as_deref());
    let authoritative = credential_source_authoritative(source.as_deref());
    EnvCredentialConfig {
        enabled,
        authoritative,
        testnet: EnvCredentialPair {
            api_key: env_secret_value("BINANCE_TESTNET_API_KEY"),
            api_secret: env_secret_value("BINANCE_TESTNET_API_SECRET_KEY"),
        },
        mainnet: EnvCredentialPair {
            api_key: env_secret_value("BINANCE_MAINNET_API_KEY"),
            api_secret: env_secret_value("BINANCE_MAINNET_API_SECRET_KEY"),
        },
    }
}

fn credential_source_enabled(source: Option<&str>, compatibility_alias: Option<&str>) -> bool {
    match source {
        Some(value) => value.trim().eq_ignore_ascii_case("env"),
        None => compatibility_alias
            .map(|value| value.trim().eq_ignore_ascii_case("true"))
            .unwrap_or(false),
    }
}

fn credential_source_authoritative(source: Option<&str>) -> bool {
    source
        .map(|value| value.trim().eq_ignore_ascii_case("env"))
        .unwrap_or(false)
}

fn env_secret_value(name: &str) -> Option<String> {
    env::var(name).ok().and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed == "..." {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn ensure_database_parent(database_url: &str) -> anyhow::Result<()> {
    if let Some(path) = database_url.strip_prefix("sqlite://") {
        let parent = Path::new(path)
            .parent()
            .context("missing sqlite parent directory")?;
        std::fs::create_dir_all(parent).context("creating sqlite parent directory")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{credential_source_authoritative, credential_source_enabled};

    #[test]
    fn explicit_env_source_is_authoritative() {
        assert!(credential_source_enabled(Some("env"), Some("false")));
        assert!(credential_source_authoritative(Some("env")));
        assert!(!credential_source_enabled(
            Some("secure_store"),
            Some("true")
        ));
        assert!(!credential_source_authoritative(Some("secure_store")));
    }

    #[test]
    fn compatibility_alias_only_applies_when_source_is_unset() {
        assert!(credential_source_enabled(None, Some("true")));
        assert!(!credential_source_enabled(None, Some("false")));
        assert!(!credential_source_enabled(None, None));
    }
}
