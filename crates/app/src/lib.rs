mod error;
mod events;
mod history;
mod ports;
mod service;

pub use error::{AppError, AppResult};
pub use events::{AppMetadata, BootstrapPayload, OutboundEvent};
pub use ports::{
    EventPublisher, KlineRangeRequest, LiveExchangePort, LiveUserDataStream, MarketDataPort,
    MarketStream, MarketStreamEvent, MetricsPort, NoopPublisher, Repository, SecretStore,
    UnavailableLiveExchange, UnavailableSecretStore,
};
pub use service::{
    env_credential_id, now_ms, AppService, EnvCredentialConfig, EnvCredentialPair,
    LiveDependencies, ServiceOptions,
};
