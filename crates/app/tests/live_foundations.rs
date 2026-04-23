mod support;

use relxen_app::{AppMetadata, AppService, LiveDependencies, Repository, ServiceOptions};
use relxen_domain::{
    AsoMode, CreateLiveCredentialRequest, LiveBlockingReason, LiveCredentialValidationStatus,
    LiveEnvironment, LiveModePreference, LiveRuntimeState, SetLiveModePreferenceRequest, Settings,
};

use support::{
    arc, candle_with_bull_at_open_time, latest_closed_open_time, FakeLiveExchange, MockRepository,
    SequenceMarket, StaticMetrics, TestSecretStore,
};

async fn initialized_service(
    repository: std::sync::Arc<MockRepository>,
    secret_store: std::sync::Arc<TestSecretStore>,
    exchange: std::sync::Arc<FakeLiveExchange>,
) -> std::sync::Arc<AppService> {
    repository
        .save_settings(&Settings {
            aso_length: 2,
            aso_mode: AsoMode::Intrabar,
            auto_restart_on_apply: false,
            ..Settings::default()
        })
        .await
        .unwrap();
    let open_time = latest_closed_open_time(relxen_domain::Timeframe::M1);
    let history = vec![
        candle_with_bull_at_open_time(
            relxen_domain::Symbol::BtcUsdt,
            relxen_domain::Timeframe::M1,
            open_time - relxen_domain::Timeframe::M1.duration_ms(),
            40.0,
            true,
        ),
        candle_with_bull_at_open_time(
            relxen_domain::Symbol::BtcUsdt,
            relxen_domain::Timeframe::M1,
            open_time,
            40.0,
            true,
        ),
    ];
    let service = AppService::new_with_live(
        AppMetadata::default(),
        repository,
        arc(SequenceMarket::new(Vec::new(), vec![history])),
        LiveDependencies::new(secret_store, exchange),
        arc(StaticMetrics),
        arc(relxen_app::NoopPublisher),
        ServiceOptions {
            history_limit: 2,
            auto_start: false,
            ..ServiceOptions::default()
        },
    );
    service.initialize().await.unwrap();
    service
}

fn create_request() -> CreateLiveCredentialRequest {
    CreateLiveCredentialRequest {
        alias: "Testnet Read Only".to_string(),
        environment: LiveEnvironment::Testnet,
        api_key: "abcd1234efgh5678".to_string(),
        api_secret: "secret".to_string(),
    }
}

#[tokio::test]
async fn live_credential_crud_masks_secret_and_persists_metadata_only() {
    let repository = arc(MockRepository::default());
    let service = initialized_service(
        repository.clone(),
        arc(TestSecretStore::default()),
        arc(FakeLiveExchange::default()),
    )
    .await;

    let credential = service
        .create_live_credential(create_request())
        .await
        .unwrap();

    assert_eq!(credential.api_key_hint, "abcd…5678");
    assert!(credential.is_active);
    let listed = service.list_live_credentials().await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].api_key_hint, "abcd…5678");
    assert_ne!(listed[0].api_key_hint, "secret");

    let updated = service
        .update_live_credential(
            credential.id.clone(),
            relxen_domain::UpdateLiveCredentialRequest {
                alias: Some("Renamed".to_string()),
                environment: None,
                api_key: None,
                api_secret: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.alias, "Renamed");

    service
        .delete_live_credential(credential.id.clone())
        .await
        .unwrap();
    assert!(service.list_live_credentials().await.unwrap().is_empty());
}

#[tokio::test]
async fn readiness_moves_from_missing_to_ready_then_armed_and_start_blocked() {
    let repository = arc(MockRepository::default());
    let service = initialized_service(
        repository,
        arc(TestSecretStore::default()),
        arc(FakeLiveExchange::default()),
    )
    .await;

    let initial = service.live_status().await.unwrap();
    assert_eq!(initial.state, LiveRuntimeState::CredentialsMissing);
    assert!(initial
        .readiness
        .blocking_reasons
        .contains(&LiveBlockingReason::NoActiveCredential));

    let credential = service
        .create_live_credential(create_request())
        .await
        .unwrap();
    let validation = service
        .validate_live_credential(credential.id.clone())
        .await
        .unwrap();
    assert_eq!(validation.status, LiveCredentialValidationStatus::Valid);

    let ready = service.refresh_live_readiness().await.unwrap();
    assert_eq!(ready.state, LiveRuntimeState::ReadyReadOnly);
    assert!(ready.readiness.can_arm);
    assert!(!ready.execution_availability.can_execute_live);

    let armed = service.arm_live().await.unwrap();
    assert_eq!(armed.state, LiveRuntimeState::ArmedReadOnly);
    assert_eq!(armed.mode_preference, LiveModePreference::LiveReadOnly);

    let start = service.live_start_check().await.unwrap();
    assert!(!start.allowed);
    assert!(start
        .blocking_reasons
        .contains(&LiveBlockingReason::ExecutionNotImplemented));

    let paper = service
        .set_live_mode_preference(SetLiveModePreferenceRequest {
            mode_preference: LiveModePreference::Paper,
        })
        .await
        .unwrap();
    assert!(!paper.armed);
}

#[tokio::test]
async fn failed_validation_blocks_readiness() {
    let repository = arc(MockRepository::default());
    let exchange = FakeLiveExchange {
        validation_status: LiveCredentialValidationStatus::InvalidApiKey,
        ..FakeLiveExchange::default()
    };
    let service =
        initialized_service(repository, arc(TestSecretStore::default()), arc(exchange)).await;

    let credential = service
        .create_live_credential(create_request())
        .await
        .unwrap();
    let validation = service
        .validate_live_credential(credential.id.clone())
        .await
        .unwrap();
    assert_eq!(
        validation.status,
        LiveCredentialValidationStatus::InvalidApiKey
    );

    let readiness = service.refresh_live_readiness().await.unwrap();
    assert_eq!(readiness.state, LiveRuntimeState::ValidationFailed);
    assert!(readiness
        .readiness
        .blocking_reasons
        .contains(&LiveBlockingReason::ValidationFailed));
}

#[tokio::test]
async fn secure_store_unavailable_is_typed_and_paper_bootstrap_still_works() {
    let repository = arc(MockRepository::default());
    let service = initialized_service(
        repository,
        arc(TestSecretStore::unavailable()),
        arc(FakeLiveExchange::default()),
    )
    .await;

    let error = service
        .create_live_credential(create_request())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        relxen_app::AppError::SecureStoreUnavailable(_)
    ));
    assert!(service.get_bootstrap().await.is_ok());
}
