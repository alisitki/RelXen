use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, Query, State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use tower_http::services::{ServeDir, ServeFile};

use relxen_app::{AppError, AppService, OutboundEvent};
use relxen_domain::{
    CreateLiveCredentialRequest, DisarmLiveModeRequest, LiveAutoExecutorRequest,
    LiveCancelAllRequest, LiveCancelRequest, LiveCredentialId, LiveExecutionRequest,
    LiveFlattenRequest, LiveKillSwitchRequest, LiveOrderType, LiveRiskProfile,
    SetLiveModePreferenceRequest, Settings, UpdateLiveCredentialRequest,
};
use relxen_infra::EventBus;

#[derive(Clone)]
pub struct RouterState {
    pub service: Arc<AppService>,
    pub event_bus: EventBus,
}

#[derive(Debug, Clone, Deserialize)]
struct LiveCancelOrderBody {
    #[serde(default)]
    order_ref: Option<String>,
    #[serde(default)]
    confirm_testnet: bool,
    #[serde(default)]
    confirm_mainnet_canary: bool,
    #[serde(default)]
    confirmation_text: Option<String>,
}

pub fn build_router(state: RouterState, frontend_dist: std::path::PathBuf) -> Router {
    let index_file = frontend_dist.join("index.html");
    Router::new()
        .route("/api/health", get(health))
        .route("/api/bootstrap", get(get_bootstrap))
        .route("/api/settings", get(get_settings).put(put_settings))
        .route("/api/runtime/start", post(start_runtime))
        .route("/api/runtime/stop", post(stop_runtime))
        .route("/api/paper/close-all", post(close_all))
        .route("/api/paper/reset", post(reset_paper))
        .route("/api/trades", get(list_trades))
        .route("/api/signals", get(list_signals))
        .route("/api/logs", get(list_logs))
        .route("/api/live/status", get(live_status))
        .route(
            "/api/live/credentials",
            get(list_live_credentials).post(create_live_credential),
        )
        .route(
            "/api/live/credentials/:credential_id",
            axum::routing::put(update_live_credential).delete(delete_live_credential),
        )
        .route(
            "/api/live/credentials/:credential_id/select",
            post(select_live_credential),
        )
        .route(
            "/api/live/credentials/:credential_id/validate",
            post(validate_live_credential),
        )
        .route("/api/live/readiness", get(live_readiness))
        .route("/api/live/readiness/refresh", post(refresh_live_readiness))
        .route("/api/live/arm", post(arm_live))
        .route("/api/live/disarm", post(disarm_live))
        .route("/api/live/start-check", post(live_start_check))
        .route("/api/live/mode", post(set_live_mode))
        .route("/api/live/shadow/start", post(start_live_shadow))
        .route("/api/live/shadow/stop", post(stop_live_shadow))
        .route("/api/live/shadow/refresh", post(refresh_live_shadow))
        .route("/api/live/intent/preview", get(live_intent_preview))
        .route("/api/live/preflight", post(run_live_preflight))
        .route("/api/live/preflights", get(list_live_preflights))
        .route("/api/live/auto/start", post(start_live_auto))
        .route("/api/live/auto/stop", post(stop_live_auto))
        .route("/api/live/mainnet-auto/status", get(mainnet_auto_status))
        .route(
            "/api/live/mainnet-auto/dry-run/start",
            post(start_mainnet_auto_dry_run),
        )
        .route(
            "/api/live/mainnet-auto/dry-run/stop",
            post(stop_mainnet_auto_dry_run),
        )
        .route(
            "/api/live/mainnet-auto/start",
            post(start_mainnet_auto_live),
        )
        .route("/api/live/mainnet-auto/stop", post(stop_mainnet_auto))
        .route(
            "/api/live/mainnet-auto/decisions",
            get(list_mainnet_auto_decisions),
        )
        .route(
            "/api/live/mainnet-auto/lessons/latest",
            get(latest_mainnet_auto_lessons),
        )
        .route(
            "/api/live/mainnet-auto/risk-budget",
            get(get_mainnet_auto_risk_budget).put(put_mainnet_auto_risk_budget),
        )
        .route(
            "/api/live/mainnet-auto/export-evidence",
            post(export_mainnet_auto_evidence),
        )
        .route(
            "/api/live/drill/auto/replay-latest-signal",
            post(replay_latest_auto_signal_drill),
        )
        .route(
            "/api/live/kill-switch/engage",
            post(engage_live_kill_switch),
        )
        .route(
            "/api/live/kill-switch/release",
            post(release_live_kill_switch),
        )
        .route(
            "/api/live/risk-profile",
            axum::routing::put(configure_live_risk_profile),
        )
        .route("/api/live/execute", post(execute_live))
        .route("/api/live/orders", get(list_live_orders))
        .route("/api/live/fills", get(list_live_fills))
        .route(
            "/api/live/orders/:order_ref/cancel",
            post(cancel_live_order),
        )
        .route("/api/live/cancel-all", post(cancel_all_live_orders))
        .route("/api/live/flatten", post(flatten_live_position))
        .route("/api/ws", get(ws_upgrade))
        .with_state(state)
        .fallback_service(
            ServeDir::new(frontend_dist).not_found_service(ServeFile::new(index_file)),
        )
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn get_bootstrap(
    State(state): State<RouterState>,
) -> Result<Json<relxen_app::BootstrapPayload>, ApiError> {
    Ok(Json(state.service.get_bootstrap().await?))
}

async fn get_settings(State(state): State<RouterState>) -> Result<Json<Settings>, ApiError> {
    Ok(Json(state.service.get_settings().await?))
}

async fn put_settings(
    State(state): State<RouterState>,
    Json(payload): Json<Settings>,
) -> Result<Json<relxen_app::BootstrapPayload>, ApiError> {
    Ok(Json(
        Arc::clone(&state.service).update_settings(payload).await?,
    ))
}

async fn start_runtime(
    State(state): State<RouterState>,
) -> Result<Json<relxen_domain::RuntimeStatus>, ApiError> {
    Ok(Json(Arc::clone(&state.service).start_runtime().await?))
}

async fn stop_runtime(
    State(state): State<RouterState>,
) -> Result<Json<relxen_domain::RuntimeStatus>, ApiError> {
    Ok(Json(state.service.stop_runtime().await?))
}

async fn close_all(
    State(state): State<RouterState>,
) -> Result<Json<relxen_app::BootstrapPayload>, ApiError> {
    Ok(Json(state.service.close_all().await?))
}

async fn reset_paper(
    State(state): State<RouterState>,
) -> Result<Json<relxen_app::BootstrapPayload>, ApiError> {
    Ok(Json(state.service.reset_paper().await?))
}

#[derive(Debug, Deserialize)]
struct LimitQuery {
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct LiveAutoDrillRequest {
    confirm_testnet_drill: bool,
}

async fn list_trades(
    State(state): State<RouterState>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<Vec<relxen_domain::Trade>>, ApiError> {
    Ok(Json(
        state
            .service
            .list_trades(query.limit.unwrap_or(100))
            .await?,
    ))
}

async fn list_signals(
    State(state): State<RouterState>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<Vec<relxen_domain::SignalEvent>>, ApiError> {
    Ok(Json(
        state
            .service
            .list_signals(query.limit.unwrap_or(100))
            .await?,
    ))
}

async fn list_logs(
    State(state): State<RouterState>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<Vec<relxen_domain::LogEvent>>, ApiError> {
    Ok(Json(
        state.service.list_logs(query.limit.unwrap_or(100)).await?,
    ))
}

async fn live_status(
    State(state): State<RouterState>,
) -> Result<Json<relxen_domain::LiveStatusSnapshot>, ApiError> {
    Ok(Json(state.service.live_status().await?))
}

async fn list_live_credentials(
    State(state): State<RouterState>,
) -> Result<Json<Vec<relxen_domain::LiveCredentialSummary>>, ApiError> {
    Ok(Json(state.service.list_live_credentials().await?))
}

async fn create_live_credential(
    State(state): State<RouterState>,
    Json(payload): Json<CreateLiveCredentialRequest>,
) -> Result<Json<relxen_domain::LiveCredentialSummary>, ApiError> {
    Ok(Json(state.service.create_live_credential(payload).await?))
}

async fn update_live_credential(
    State(state): State<RouterState>,
    Path(credential_id): Path<String>,
    Json(payload): Json<UpdateLiveCredentialRequest>,
) -> Result<Json<relxen_domain::LiveCredentialSummary>, ApiError> {
    Ok(Json(
        state
            .service
            .update_live_credential(LiveCredentialId::new(credential_id), payload)
            .await?,
    ))
}

async fn delete_live_credential(
    State(state): State<RouterState>,
    Path(credential_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    state
        .service
        .delete_live_credential(LiveCredentialId::new(credential_id))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn select_live_credential(
    State(state): State<RouterState>,
    Path(credential_id): Path<String>,
) -> Result<Json<relxen_domain::LiveStatusSnapshot>, ApiError> {
    Ok(Json(
        state
            .service
            .select_live_credential(LiveCredentialId::new(credential_id))
            .await?,
    ))
}

async fn validate_live_credential(
    State(state): State<RouterState>,
    Path(credential_id): Path<String>,
) -> Result<Json<relxen_domain::LiveCredentialValidationResult>, ApiError> {
    Ok(Json(
        state
            .service
            .validate_live_credential(LiveCredentialId::new(credential_id))
            .await?,
    ))
}

async fn live_readiness(
    State(state): State<RouterState>,
) -> Result<Json<relxen_domain::LiveReadinessSnapshot>, ApiError> {
    Ok(Json(state.service.live_status().await?.readiness))
}

async fn refresh_live_readiness(
    State(state): State<RouterState>,
) -> Result<Json<relxen_domain::LiveStatusSnapshot>, ApiError> {
    Ok(Json(state.service.refresh_live_readiness().await?))
}

async fn arm_live(
    State(state): State<RouterState>,
) -> Result<Json<relxen_domain::LiveStatusSnapshot>, ApiError> {
    Ok(Json(state.service.arm_live().await?))
}

async fn disarm_live(
    State(state): State<RouterState>,
    body: Option<Json<DisarmLiveModeRequest>>,
) -> Result<Json<relxen_domain::LiveStatusSnapshot>, ApiError> {
    Ok(Json(
        state
            .service
            .disarm_live(
                body.map(|Json(payload)| payload)
                    .unwrap_or(DisarmLiveModeRequest { reason: None }),
            )
            .await?,
    ))
}

async fn live_start_check(
    State(state): State<RouterState>,
) -> Result<Json<relxen_domain::LiveStartCheck>, ApiError> {
    Ok(Json(state.service.live_start_check().await?))
}

async fn set_live_mode(
    State(state): State<RouterState>,
    Json(payload): Json<SetLiveModePreferenceRequest>,
) -> Result<Json<relxen_domain::LiveStatusSnapshot>, ApiError> {
    Ok(Json(state.service.set_live_mode_preference(payload).await?))
}

async fn start_live_shadow(
    State(state): State<RouterState>,
) -> Result<Json<relxen_domain::LiveStatusSnapshot>, ApiError> {
    Ok(Json(Arc::clone(&state.service).start_live_shadow().await?))
}

async fn stop_live_shadow(
    State(state): State<RouterState>,
) -> Result<Json<relxen_domain::LiveStatusSnapshot>, ApiError> {
    Ok(Json(state.service.stop_live_shadow().await?))
}

async fn refresh_live_shadow(
    State(state): State<RouterState>,
) -> Result<Json<relxen_domain::LiveStatusSnapshot>, ApiError> {
    Ok(Json(state.service.refresh_live_shadow().await?))
}

#[derive(Debug, Deserialize)]
struct IntentPreviewQuery {
    order_type: Option<String>,
    limit_price: Option<String>,
}

async fn live_intent_preview(
    State(state): State<RouterState>,
    Query(query): Query<IntentPreviewQuery>,
) -> Result<Json<relxen_domain::LiveOrderPreview>, ApiError> {
    let order_type = match query.order_type.as_deref().unwrap_or("MARKET") {
        "LIMIT" | "limit" => LiveOrderType::Limit,
        _ => LiveOrderType::Market,
    };
    let limit_price = query
        .limit_price
        .as_deref()
        .map(rust_decimal::Decimal::from_str_exact)
        .transpose()
        .map_err(|error| AppError::Validation(format!("invalid limit_price: {error}")))?;
    Ok(Json(
        state
            .service
            .build_live_intent_preview(order_type, limit_price)
            .await?,
    ))
}

async fn run_live_preflight(
    State(state): State<RouterState>,
) -> Result<Json<relxen_domain::LiveOrderPreflightResult>, ApiError> {
    Ok(Json(state.service.run_live_preflight().await?))
}

async fn list_live_preflights(
    State(state): State<RouterState>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<Vec<relxen_domain::LiveOrderPreflightResult>>, ApiError> {
    Ok(Json(
        state
            .service
            .list_live_preflights(query.limit.unwrap_or(50))
            .await?,
    ))
}

async fn start_live_auto(
    State(state): State<RouterState>,
    body: Option<Json<LiveAutoExecutorRequest>>,
) -> Result<Json<relxen_domain::LiveStatusSnapshot>, ApiError> {
    Ok(Json(
        state
            .service
            .start_live_auto_executor(body.map(|Json(payload)| payload).unwrap_or(
                LiveAutoExecutorRequest {
                    confirm_testnet_auto: false,
                },
            ))
            .await?,
    ))
}

async fn stop_live_auto(
    State(state): State<RouterState>,
) -> Result<Json<relxen_domain::LiveStatusSnapshot>, ApiError> {
    Ok(Json(state.service.stop_live_auto_executor().await?))
}

async fn mainnet_auto_status(
    State(state): State<RouterState>,
) -> Result<Json<relxen_domain::MainnetAutoStatus>, ApiError> {
    Ok(Json(state.service.mainnet_auto_status().await?))
}

async fn start_mainnet_auto_dry_run(
    State(state): State<RouterState>,
) -> Result<Json<relxen_domain::MainnetAutoStatus>, ApiError> {
    Ok(Json(state.service.start_mainnet_auto_dry_run().await?))
}

async fn stop_mainnet_auto_dry_run(
    State(state): State<RouterState>,
) -> Result<Json<relxen_domain::MainnetAutoStatus>, ApiError> {
    Ok(Json(state.service.stop_mainnet_auto_dry_run().await?))
}

async fn start_mainnet_auto_live(
    State(state): State<RouterState>,
) -> Result<Json<relxen_domain::MainnetAutoStatus>, ApiError> {
    Ok(Json(state.service.start_mainnet_auto_live().await?))
}

async fn stop_mainnet_auto(
    State(state): State<RouterState>,
) -> Result<Json<relxen_domain::MainnetAutoStatus>, ApiError> {
    Ok(Json(state.service.stop_mainnet_auto().await?))
}

async fn list_mainnet_auto_decisions(
    State(state): State<RouterState>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<Vec<relxen_domain::MainnetAutoDecisionEvent>>, ApiError> {
    Ok(Json(
        state
            .service
            .list_mainnet_auto_decisions(query.limit.unwrap_or(100))
            .await?,
    ))
}

async fn latest_mainnet_auto_lessons(
    State(state): State<RouterState>,
) -> Result<Json<Option<relxen_domain::MainnetAutoLessonReport>>, ApiError> {
    Ok(Json(state.service.latest_mainnet_auto_lessons().await?))
}

async fn get_mainnet_auto_risk_budget(
    State(state): State<RouterState>,
) -> Result<Json<relxen_domain::MainnetAutoRiskBudget>, ApiError> {
    Ok(Json(state.service.mainnet_auto_risk_budget().await?))
}

async fn put_mainnet_auto_risk_budget(
    State(state): State<RouterState>,
    Json(payload): Json<relxen_domain::MainnetAutoRiskBudget>,
) -> Result<Json<relxen_domain::MainnetAutoRiskBudget>, ApiError> {
    Ok(Json(
        state
            .service
            .configure_mainnet_auto_risk_budget(payload)
            .await?,
    ))
}

async fn export_mainnet_auto_evidence(
    State(state): State<RouterState>,
) -> Result<Json<relxen_domain::MainnetAutoEvidenceExportResult>, ApiError> {
    Ok(Json(state.service.export_mainnet_auto_evidence().await?))
}

async fn replay_latest_auto_signal_drill(
    State(state): State<RouterState>,
    body: Option<Json<LiveAutoDrillRequest>>,
) -> Result<Json<relxen_domain::LiveStatusSnapshot>, ApiError> {
    let confirmed = body
        .map(|Json(payload)| payload.confirm_testnet_drill)
        .unwrap_or(false);
    if !confirmed {
        return Err(ApiError::from(AppError::Validation(
            "TESTNET auto drill requires explicit confirmation.".to_string(),
        )));
    }
    Ok(Json(state.service.drill_replay_latest_auto_signal().await?))
}

async fn engage_live_kill_switch(
    State(state): State<RouterState>,
    body: Option<Json<LiveKillSwitchRequest>>,
) -> Result<Json<relxen_domain::LiveStatusSnapshot>, ApiError> {
    Ok(Json(
        state
            .service
            .engage_live_kill_switch(
                body.map(|Json(payload)| payload)
                    .unwrap_or(LiveKillSwitchRequest { reason: None }),
            )
            .await?,
    ))
}

async fn release_live_kill_switch(
    State(state): State<RouterState>,
    body: Option<Json<LiveKillSwitchRequest>>,
) -> Result<Json<relxen_domain::LiveStatusSnapshot>, ApiError> {
    Ok(Json(
        state
            .service
            .release_live_kill_switch(
                body.map(|Json(payload)| payload)
                    .unwrap_or(LiveKillSwitchRequest { reason: None }),
            )
            .await?,
    ))
}

async fn configure_live_risk_profile(
    State(state): State<RouterState>,
    Json(payload): Json<LiveRiskProfile>,
) -> Result<Json<relxen_domain::LiveStatusSnapshot>, ApiError> {
    Ok(Json(
        state.service.configure_live_risk_profile(payload).await?,
    ))
}

async fn execute_live(
    State(state): State<RouterState>,
    body: Option<Json<LiveExecutionRequest>>,
) -> Result<Json<relxen_domain::LiveExecutionResult>, ApiError> {
    Ok(Json(
        state
            .service
            .execute_live_current_preview(body.map(|Json(payload)| payload).unwrap_or(
                LiveExecutionRequest {
                    intent_id: None,
                    confirm_testnet: false,
                    confirm_mainnet_canary: false,
                    confirmation_text: None,
                },
            ))
            .await?,
    ))
}

async fn cancel_live_order(
    State(state): State<RouterState>,
    Path(order_ref): Path<String>,
    body: Option<Json<LiveCancelOrderBody>>,
) -> Result<Json<relxen_domain::LiveCancelResult>, ApiError> {
    let payload = body.map(|Json(payload)| payload);
    if let Some(body_order_ref) = payload
        .as_ref()
        .and_then(|payload| payload.order_ref.as_deref())
    {
        if body_order_ref != order_ref {
            return Err(ApiError::from(AppError::Validation(
                "cancel order_ref in request body must match the route path".to_string(),
            )));
        }
    }
    let confirm_testnet = payload
        .as_ref()
        .map(|payload| payload.confirm_testnet)
        .unwrap_or(false);
    let confirm_mainnet_canary = payload
        .as_ref()
        .map(|payload| payload.confirm_mainnet_canary)
        .unwrap_or(false);
    let confirmation_text = payload.and_then(|payload| payload.confirmation_text);
    Ok(Json(
        state
            .service
            .cancel_live_order(LiveCancelRequest {
                order_ref,
                confirm_testnet,
                confirm_mainnet_canary,
                confirmation_text,
            })
            .await?,
    ))
}

async fn cancel_all_live_orders(
    State(state): State<RouterState>,
    body: Option<Json<LiveCancelAllRequest>>,
) -> Result<Json<Vec<relxen_domain::LiveCancelResult>>, ApiError> {
    Ok(Json(
        state
            .service
            .cancel_all_live_orders(body.map(|Json(payload)| payload).unwrap_or(
                LiveCancelAllRequest {
                    confirm_testnet: false,
                    confirm_mainnet_canary: false,
                    confirmation_text: None,
                },
            ))
            .await?,
    ))
}

async fn flatten_live_position(
    State(state): State<RouterState>,
    body: Option<Json<LiveFlattenRequest>>,
) -> Result<Json<relxen_domain::LiveFlattenResult>, ApiError> {
    Ok(Json(
        state
            .service
            .flatten_live_position(body.map(|Json(payload)| payload).unwrap_or(
                LiveFlattenRequest {
                    confirm_testnet: false,
                    confirm_mainnet_canary: false,
                    confirmation_text: None,
                },
            ))
            .await?,
    ))
}

async fn list_live_orders(
    State(state): State<RouterState>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<Vec<relxen_domain::LiveOrderRecord>>, ApiError> {
    Ok(Json(
        state
            .service
            .list_live_orders(query.limit.unwrap_or(50))
            .await?,
    ))
}

async fn list_live_fills(
    State(state): State<RouterState>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<Vec<relxen_domain::LiveFillRecord>>, ApiError> {
    Ok(Json(
        state
            .service
            .list_live_fills(query.limit.unwrap_or(100))
            .await?,
    ))
}

async fn ws_upgrade(ws: WebSocketUpgrade, State(state): State<RouterState>) -> Response {
    ws.on_upgrade(move |socket| websocket_session(socket, state))
}

async fn websocket_session(mut socket: WebSocket, state: RouterState) {
    if let Ok(snapshot) = state.service.get_bootstrap().await {
        if send_event(&mut socket, OutboundEvent::Snapshot(Box::new(snapshot)))
            .await
            .is_err()
        {
            return;
        }
    }

    let mut receiver = state.event_bus.subscribe();
    loop {
        match receiver.recv().await {
            Ok(event) => {
                if send_event(&mut socket, event).await.is_err() {
                    break;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                if send_event(
                    &mut socket,
                    OutboundEvent::ResyncRequired {
                        reason: "client lagged behind server event bus".to_string(),
                    },
                )
                .await
                .is_err()
                {
                    break;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
        }
    }
}

async fn send_event(socket: &mut WebSocket, event: OutboundEvent) -> Result<(), ()> {
    let payload = serde_json::to_string(&event).map_err(|_| ())?;
    socket.send(Message::Text(payload)).await.map_err(|_| ())
}

struct ApiError(AppError);

impl From<AppError> for ApiError {
    fn from(value: AppError) -> Self {
        Self(value)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let kind = app_error_kind(&self.0);
        let status = match &self.0 {
            AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::History(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::SecureStoreUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            AppError::Live(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::Exchange(_) => StatusCode::BAD_GATEWAY,
            AppError::Other(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = Json(serde_json::json!({
            "error": self.0.to_string(),
            "kind": kind,
            "status": status.as_u16()
        }));
        (status, body).into_response()
    }
}

fn app_error_kind(error: &AppError) -> &'static str {
    match error {
        AppError::Validation(_) => "validation",
        AppError::History(_) => "history",
        AppError::Conflict(_) => "conflict",
        AppError::NotFound(_) => "not_found",
        AppError::SecureStoreUnavailable(_) => "secure_store_unavailable",
        AppError::Live(_) => "live",
        AppError::Exchange(_) => "exchange",
        AppError::Other(_) => "internal",
    }
}
