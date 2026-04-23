use std::env;
use std::path::{Path, PathBuf};

use anyhow::Context;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_addr: String,
    pub database_url: String,
    pub frontend_dist: PathBuf,
    pub log_level: String,
    pub auto_start: bool,
    pub enable_mainnet_canary_execution: bool,
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

        ensure_database_parent(&database_url)?;

        Ok(Self {
            bind_addr,
            database_url,
            frontend_dist: PathBuf::from(frontend_dist),
            log_level,
            auto_start,
            enable_mainnet_canary_execution,
        })
    }
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
