use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::Context;
use tracing_subscriber::EnvFilter;

use relxen_app::{
    env_credential_id, AppMetadata, AppService, EnvCredentialConfig, LiveDependencies,
    ServiceOptions,
};
use relxen_domain::{LiveCredentialSecret, LiveEnvironment};
use relxen_infra::{
    BinanceLiveReadOnly, BinanceMarketData, EnvOverlaySecretStore, EventBus, OsSecretStore,
    SqliteRepository, SystemMetricsCollector,
};
use relxen_server::{build_router, RouterState, ServerConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = ServerConfig::from_env()?;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&config.log_level))
        .with_target(true)
        .init();

    let repository = Arc::new(SqliteRepository::connect(&config.database_url).await?);
    let event_bus = EventBus::new(1024);
    let base_secret_store = Arc::new(OsSecretStore);
    let secret_store = Arc::new(EnvOverlaySecretStore::new(
        base_secret_store,
        env_secret_map(&config.env_credentials),
    ));
    let service = AppService::new_with_live(
        AppMetadata::default(),
        repository,
        Arc::new(BinanceMarketData::default()),
        LiveDependencies::new(secret_store, Arc::new(BinanceLiveReadOnly::default())),
        Arc::new(SystemMetricsCollector::default()),
        Arc::new(event_bus.clone()),
        ServiceOptions {
            auto_start: config.auto_start,
            enable_mainnet_canary_execution: config.enable_mainnet_canary_execution,
            enable_testnet_drill_helpers: config.enable_testnet_drill_helpers,
            env_credentials: config.env_credentials,
            mainnet_auto_config: config.mainnet_auto,
            ..ServiceOptions::default()
        },
    );
    service
        .initialize()
        .await
        .context("initializing application service")?;

    let router = build_router(RouterState { service, event_bus }, config.frontend_dist);

    let listener = tokio::net::TcpListener::bind(&config.bind_addr)
        .await
        .with_context(|| format!("binding {}", config.bind_addr))?;
    tracing::info!("listening on {}", config.bind_addr);
    axum::serve(listener, router)
        .await
        .context("serving HTTP")?;
    Ok(())
}

fn env_secret_map(config: &EnvCredentialConfig) -> BTreeMap<String, LiveCredentialSecret> {
    let mut secrets = BTreeMap::new();
    if !config.enabled {
        return secrets;
    }
    if let (Some(api_key), Some(api_secret)) = (
        config.testnet.api_key.clone(),
        config.testnet.api_secret.clone(),
    ) {
        secrets.insert(
            env_credential_id(LiveEnvironment::Testnet)
                .as_str()
                .to_string(),
            LiveCredentialSecret {
                api_key,
                api_secret,
            },
        );
    }
    if let (Some(api_key), Some(api_secret)) = (
        config.mainnet.api_key.clone(),
        config.mainnet.api_secret.clone(),
    ) {
        secrets.insert(
            env_credential_id(LiveEnvironment::Mainnet)
                .as_str()
                .to_string(),
            LiveCredentialSecret {
                api_key,
                api_secret,
            },
        );
    }
    secrets
}
