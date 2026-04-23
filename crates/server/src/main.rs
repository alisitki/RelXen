use std::sync::Arc;

use anyhow::Context;
use tracing_subscriber::EnvFilter;

use relxen_app::{AppMetadata, AppService, LiveDependencies, ServiceOptions};
use relxen_infra::{
    BinanceLiveReadOnly, BinanceMarketData, EventBus, OsSecretStore, SqliteRepository,
    SystemMetricsCollector,
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
    let service = AppService::new_with_live(
        AppMetadata::default(),
        repository,
        Arc::new(BinanceMarketData::default()),
        LiveDependencies::new(
            Arc::new(OsSecretStore),
            Arc::new(BinanceLiveReadOnly::default()),
        ),
        Arc::new(SystemMetricsCollector::default()),
        Arc::new(event_bus.clone()),
        ServiceOptions {
            auto_start: config.auto_start,
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
