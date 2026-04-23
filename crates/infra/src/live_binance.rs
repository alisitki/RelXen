use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::Context;
use async_trait::async_trait;
use futures::StreamExt;
use hmac::{Hmac, Mac};
use reqwest::Method;
use serde::Deserialize;
use sha2::Sha256;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};
use url::form_urlencoded::Serializer;

use relxen_app::{now_ms, AppError, AppResult, LiveExchangePort, LiveUserDataStream};
use relxen_domain::{
    LiveAccountShadow, LiveAccountSnapshot, LiveAssetBalance, LiveCredentialId,
    LiveCredentialSecret, LiveCredentialValidationResult, LiveCredentialValidationStatus,
    LiveEnvironment, LiveFillRecord, LiveOrderRecord, LiveOrderSide, LiveOrderStatus,
    LiveOrderType, LivePositionSnapshot, LiveShadowBalance, LiveShadowOrder, LiveShadowPosition,
    LiveSymbolFilterSummary, LiveSymbolRules, LiveUserDataEvent, QuoteAsset, Symbol,
};

const MAINNET_REST_BASE: &str = "https://fapi.binance.com";
const TESTNET_REST_BASE: &str = "https://testnet.binancefuture.com";
const MAINNET_WS_BASE: &str = "wss://fstream.binance.com/ws";
const TESTNET_WS_BASE: &str = "wss://stream.binancefuture.com/ws";
const RECV_WINDOW_MS: i64 = 5_000;

#[derive(Debug, Clone)]
pub struct BinanceLiveReadOnly {
    client: Arc<reqwest::Client>,
    mainnet_base: Arc<str>,
    testnet_base: Arc<str>,
    mainnet_ws_base: Arc<str>,
    testnet_ws_base: Arc<str>,
}

impl Default for BinanceLiveReadOnly {
    fn default() -> Self {
        Self {
            client: Arc::new(reqwest::Client::new()),
            mainnet_base: Arc::from(MAINNET_REST_BASE),
            testnet_base: Arc::from(TESTNET_REST_BASE),
            mainnet_ws_base: Arc::from(MAINNET_WS_BASE),
            testnet_ws_base: Arc::from(TESTNET_WS_BASE),
        }
    }
}

impl BinanceLiveReadOnly {
    #[cfg(test)]
    fn with_bases(
        client: reqwest::Client,
        mainnet_base: impl Into<String>,
        testnet_base: impl Into<String>,
    ) -> Self {
        Self {
            client: Arc::new(client),
            mainnet_base: Arc::from(mainnet_base.into()),
            testnet_base: Arc::from(testnet_base.into()),
            mainnet_ws_base: Arc::from(MAINNET_WS_BASE),
            testnet_ws_base: Arc::from(TESTNET_WS_BASE),
        }
    }

    fn base_url(&self, environment: LiveEnvironment) -> &str {
        match environment {
            LiveEnvironment::Mainnet => self.mainnet_base.as_ref(),
            LiveEnvironment::Testnet => self.testnet_base.as_ref(),
        }
    }

    fn endpoint(&self, environment: LiveEnvironment, path: &str) -> String {
        format!(
            "{}{}",
            self.base_url(environment).trim_end_matches('/'),
            path
        )
    }

    fn websocket_url(&self, environment: LiveEnvironment, listen_key: &str) -> String {
        let base = match environment {
            LiveEnvironment::Mainnet => self.mainnet_ws_base.as_ref(),
            LiveEnvironment::Testnet => self.testnet_ws_base.as_ref(),
        };
        format!("{}/{}", base.trim_end_matches('/'), listen_key)
    }
}

#[async_trait]
impl LiveExchangePort for BinanceLiveReadOnly {
    async fn validate_credentials(
        &self,
        environment: LiveEnvironment,
        credential_id: &LiveCredentialId,
        secret: &LiveCredentialSecret,
    ) -> AppResult<LiveCredentialValidationResult> {
        info!(
            event = "credential_validation_started",
            credential_id = %credential_id,
            environment = %environment,
            "validating Binance USD-M read-only credentials"
        );
        let validated_at = now_ms();
        match request_signed_account(
            self.client.as_ref(),
            &self.endpoint(environment, "/fapi/v2/account"),
            secret,
        )
        .await
        {
            Ok(_) => Ok(LiveCredentialValidationResult {
                credential_id: credential_id.clone(),
                environment,
                status: LiveCredentialValidationStatus::Valid,
                validated_at,
                message: None,
            }),
            Err(error) => {
                let status = classify_validation_error(&error);
                Ok(LiveCredentialValidationResult {
                    credential_id: credential_id.clone(),
                    environment,
                    status,
                    validated_at,
                    message: Some(error.to_string()),
                })
            }
        }
    }

    async fn fetch_account_snapshot(
        &self,
        environment: LiveEnvironment,
        secret: &LiveCredentialSecret,
    ) -> AppResult<LiveAccountSnapshot> {
        let account = request_signed_account(
            self.client.as_ref(),
            &self.endpoint(environment, "/fapi/v2/account"),
            secret,
        )
        .await?;
        Ok(account.into_snapshot(environment, now_ms()))
    }

    async fn fetch_symbol_rules(
        &self,
        environment: LiveEnvironment,
        symbol: Symbol,
    ) -> AppResult<LiveSymbolRules> {
        let url = self.endpoint(environment, "/fapi/v1/exchangeInfo");
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|error| AppError::Exchange(format!("network_error: {error}")))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .context("reading Binance exchangeInfo body")?;
        if !status.is_success() {
            return Err(AppError::Exchange(format!(
                "exchange_error: HTTP {status}: {body}"
            )));
        }
        let exchange_info: BinanceExchangeInfo = serde_json::from_str(&body)
            .map_err(|error| AppError::Exchange(format!("response_decode_error: {error}")))?;
        exchange_info
            .symbols
            .into_iter()
            .find(|item| item.symbol == symbol.as_str())
            .ok_or_else(|| AppError::Exchange(format!("unsupported_symbol: {symbol}")))?
            .into_rules(environment, symbol, now_ms())
    }

    async fn create_listen_key(
        &self,
        environment: LiveEnvironment,
        secret: &LiveCredentialSecret,
    ) -> AppResult<String> {
        let response: ListenKeyResponse = self
            .client
            .post(self.endpoint(environment, "/fapi/v1/listenKey"))
            .header("X-MBX-APIKEY", secret.api_key.trim())
            .send()
            .await
            .map_err(|error| AppError::Exchange(format!("network_error: {error}")))?
            .json()
            .await
            .map_err(|error| AppError::Exchange(format!("response_decode_error: {error}")))?;
        Ok(response.listen_key)
    }

    async fn keepalive_listen_key(
        &self,
        environment: LiveEnvironment,
        secret: &LiveCredentialSecret,
        listen_key: &str,
    ) -> AppResult<()> {
        let response = self
            .client
            .put(self.endpoint(environment, "/fapi/v1/listenKey"))
            .header("X-MBX-APIKEY", secret.api_key.trim())
            .query(&[("listenKey", listen_key)])
            .send()
            .await
            .map_err(|error| AppError::Exchange(format!("network_error: {error}")))?;
        ensure_success(response).await
    }

    async fn close_listen_key(
        &self,
        environment: LiveEnvironment,
        secret: &LiveCredentialSecret,
        listen_key: &str,
    ) -> AppResult<()> {
        let response = self
            .client
            .delete(self.endpoint(environment, "/fapi/v1/listenKey"))
            .header("X-MBX-APIKEY", secret.api_key.trim())
            .query(&[("listenKey", listen_key)])
            .send()
            .await
            .map_err(|error| AppError::Exchange(format!("network_error: {error}")))?;
        ensure_success(response).await
    }

    async fn subscribe_user_data(
        &self,
        environment: LiveEnvironment,
        listen_key: &str,
    ) -> AppResult<LiveUserDataStream> {
        let (stream, _) = connect_async(self.websocket_url(environment, listen_key))
            .await
            .map_err(|error| AppError::Exchange(format!("network_error: {error}")))?;
        let events = stream.filter_map(|message| async move {
            match message {
                Ok(Message::Text(text)) => Some(parse_user_data_event(&text)),
                Ok(Message::Binary(bytes)) => String::from_utf8(bytes.to_vec())
                    .ok()
                    .map(|text| parse_user_data_event(&text)),
                Ok(Message::Ping(_) | Message::Pong(_) | Message::Close(_)) => None,
                Ok(_) => None,
                Err(error) => Some(Err(AppError::Exchange(format!("network_error: {error}")))),
            }
        });
        Ok(Box::pin(events))
    }

    async fn preflight_order_test(
        &self,
        environment: LiveEnvironment,
        secret: &LiveCredentialSecret,
        payload: &BTreeMap<String, String>,
    ) -> AppResult<()> {
        let query = {
            let mut serializer = Serializer::new(String::new());
            for (key, value) in payload {
                serializer.append_pair(key, value);
            }
            serializer.append_pair("timestamp", &now_ms().to_string());
            serializer.append_pair("recvWindow", &RECV_WINDOW_MS.to_string());
            serializer.finish()
        };
        let signature = sign_query(&query, &secret.api_secret)?;
        let signed_query = format!("{query}&signature={signature}");
        let response = self
            .client
            .post(format!(
                "{}?{}",
                self.endpoint(environment, "/fapi/v1/order/test"),
                signed_query
            ))
            .header("X-MBX-APIKEY", secret.api_key.trim())
            .send()
            .await
            .map_err(|error| AppError::Exchange(format!("network_error: {error}")))?;
        ensure_success(response).await
    }

    async fn submit_order(
        &self,
        environment: LiveEnvironment,
        secret: &LiveCredentialSecret,
        payload: &BTreeMap<String, String>,
    ) -> AppResult<LiveOrderRecord> {
        if environment != LiveEnvironment::Testnet {
            return Err(AppError::Exchange(
                "execution_not_supported_on_mainnet: actual order placement is testnet-only"
                    .to_string(),
            ));
        }
        info!(
            event = "live_execute_requested",
            environment = %environment,
            symbol = payload.get("symbol").map(String::as_str).unwrap_or("unknown"),
            order_type = payload.get("type").map(String::as_str).unwrap_or("unknown"),
            "submitting Binance testnet order"
        );
        let response = self
            .signed_request_json::<BinanceOrderResponse>(
                Method::POST,
                environment,
                "/fapi/v1/order",
                secret,
                payload,
            )
            .await?;
        Ok(response.into_order_record(environment, payload.clone(), now_ms()))
    }

    async fn cancel_order(
        &self,
        environment: LiveEnvironment,
        secret: &LiveCredentialSecret,
        symbol: Symbol,
        orig_client_order_id: Option<&str>,
        order_id: Option<&str>,
    ) -> AppResult<LiveOrderRecord> {
        if environment != LiveEnvironment::Testnet {
            return Err(AppError::Exchange(
                "execution_not_supported_on_mainnet: actual order cancel is testnet-only"
                    .to_string(),
            ));
        }
        let mut payload = BTreeMap::new();
        payload.insert("symbol".to_string(), symbol.as_str().to_string());
        if let Some(orig_client_order_id) = orig_client_order_id {
            payload.insert(
                "origClientOrderId".to_string(),
                orig_client_order_id.to_string(),
            );
        }
        if let Some(order_id) = order_id {
            payload.insert("orderId".to_string(), order_id.to_string());
        }
        if !payload.contains_key("origClientOrderId") && !payload.contains_key("orderId") {
            return Err(AppError::Exchange(
                "cancel_failed: missing exchange order identifier".to_string(),
            ));
        }
        let response = self
            .signed_request_json::<BinanceOrderResponse>(
                Method::DELETE,
                environment,
                "/fapi/v1/order",
                secret,
                &payload,
            )
            .await?;
        Ok(response.into_order_record(environment, payload, now_ms()))
    }

    async fn query_order(
        &self,
        environment: LiveEnvironment,
        secret: &LiveCredentialSecret,
        symbol: Symbol,
        orig_client_order_id: Option<&str>,
        order_id: Option<&str>,
    ) -> AppResult<Option<LiveOrderRecord>> {
        let mut payload = BTreeMap::new();
        payload.insert("symbol".to_string(), symbol.as_str().to_string());
        if let Some(orig_client_order_id) = orig_client_order_id {
            payload.insert(
                "origClientOrderId".to_string(),
                orig_client_order_id.to_string(),
            );
        }
        if let Some(order_id) = order_id {
            payload.insert("orderId".to_string(), order_id.to_string());
        }
        let result = self
            .signed_request_json::<BinanceOrderResponse>(
                Method::GET,
                environment,
                "/fapi/v1/order",
                secret,
                &payload,
            )
            .await;
        match result {
            Ok(response) => Ok(Some(response.into_order_record(
                environment,
                payload,
                now_ms(),
            ))),
            Err(error) if error.to_string().contains("order_not_found") => Ok(None),
            Err(error) => Err(error),
        }
    }

    async fn list_open_orders(
        &self,
        environment: LiveEnvironment,
        secret: &LiveCredentialSecret,
        symbol: Symbol,
    ) -> AppResult<Vec<LiveOrderRecord>> {
        let mut payload = BTreeMap::new();
        payload.insert("symbol".to_string(), symbol.as_str().to_string());
        let responses = self
            .signed_request_json::<Vec<BinanceOrderResponse>>(
                Method::GET,
                environment,
                "/fapi/v1/openOrders",
                secret,
                &payload,
            )
            .await?;
        Ok(responses
            .into_iter()
            .map(|response| response.into_order_record(environment, BTreeMap::new(), now_ms()))
            .collect())
    }

    async fn list_user_trades(
        &self,
        environment: LiveEnvironment,
        secret: &LiveCredentialSecret,
        symbol: Symbol,
        limit: usize,
    ) -> AppResult<Vec<LiveFillRecord>> {
        let mut payload = BTreeMap::new();
        payload.insert("symbol".to_string(), symbol.as_str().to_string());
        payload.insert("limit".to_string(), limit.min(1_000).to_string());
        let trades = self
            .signed_request_json::<Vec<BinanceUserTrade>>(
                Method::GET,
                environment,
                "/fapi/v1/userTrades",
                secret,
                &payload,
            )
            .await?;
        Ok(trades
            .into_iter()
            .filter_map(|trade| trade.into_fill_record())
            .collect())
    }
}

impl BinanceLiveReadOnly {
    async fn signed_request_json<T>(
        &self,
        method: Method,
        environment: LiveEnvironment,
        path: &str,
        secret: &LiveCredentialSecret,
        payload: &BTreeMap<String, String>,
    ) -> AppResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let signed_query = signed_query(payload, &secret.api_secret)?;
        let url = format!("{}?{}", self.endpoint(environment, path), signed_query);
        let response = self
            .client
            .request(method, url)
            .header("X-MBX-APIKEY", secret.api_key.trim())
            .send()
            .await
            .map_err(|error| AppError::Exchange(format!("network_error: {error}")))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .context("reading Binance signed response body")?;
        if !status.is_success() {
            return Err(map_binance_error(status.as_u16(), &body));
        }
        serde_json::from_str(&body)
            .map_err(|error| AppError::Exchange(format!("response_decode_error: {error}")))
    }
}

async fn ensure_success(response: reqwest::Response) -> AppResult<()> {
    let status = response.status();
    let body = response
        .text()
        .await
        .context("reading Binance response body")?;
    if status.is_success() {
        Ok(())
    } else {
        Err(map_binance_error(status.as_u16(), &body))
    }
}

async fn request_signed_account(
    client: &reqwest::Client,
    url: &str,
    secret: &LiveCredentialSecret,
) -> AppResult<BinanceAccountResponse> {
    let timestamp = now_ms();
    let query = Serializer::new(String::new())
        .append_pair("timestamp", &timestamp.to_string())
        .append_pair("recvWindow", &RECV_WINDOW_MS.to_string())
        .finish();
    let signature = sign_query(&query, &secret.api_secret)?;
    let signed_query = format!("{query}&signature={signature}");
    let signed_url = format!("{url}?{signed_query}");

    let response = client
        .get(signed_url)
        .header("X-MBX-APIKEY", secret.api_key.trim())
        .send()
        .await
        .map_err(|error| AppError::Exchange(format!("network_error: {error}")))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .context("reading Binance signed response body")?;
    if !status.is_success() {
        warn!(
            event = "credential_validation_failed",
            status = status.as_u16(),
            "Binance signed read-only request failed"
        );
        return Err(map_binance_error(status.as_u16(), &body));
    }
    serde_json::from_str(&body)
        .map_err(|error| AppError::Exchange(format!("response_decode_error: {error}")))
}

fn sign_query(query: &str, api_secret: &str) -> AppResult<String> {
    let mut mac = Hmac::<Sha256>::new_from_slice(api_secret.as_bytes())
        .map_err(|error| AppError::Exchange(format!("invalid_signature_key: {error}")))?;
    mac.update(query.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

fn signed_query(payload: &BTreeMap<String, String>, api_secret: &str) -> AppResult<String> {
    let mut serializer = Serializer::new(String::new());
    for (key, value) in payload {
        serializer.append_pair(key, value);
    }
    serializer.append_pair("timestamp", &now_ms().to_string());
    serializer.append_pair("recvWindow", &RECV_WINDOW_MS.to_string());
    let query = serializer.finish();
    let signature = sign_query(&query, api_secret)?;
    Ok(format!("{query}&signature={signature}"))
}

fn map_binance_error(status: u16, body: &str) -> AppError {
    if let Ok(error) = serde_json::from_str::<BinanceError>(body) {
        let kind = match error.code {
            -2015 => "invalid_api_key",
            -1022 => "invalid_signature",
            -1021 => "timestamp_skew",
            -2014 => "invalid_api_key",
            -2013 => "order_not_found",
            -2011 => "cancel_failed",
            -2010 => "order_rejected",
            -4111 => "duplicate_client_order_id",
            _ if status == 401 || status == 403 => "permission_denied",
            _ => "exchange_error",
        };
        return AppError::Exchange(format!("{kind}: {}", error.msg));
    }
    AppError::Exchange(format!("exchange_error: HTTP {status}: {body}"))
}

fn classify_validation_error(error: &AppError) -> LiveCredentialValidationStatus {
    let message = error.to_string();
    if message.contains("invalid_api_key") {
        LiveCredentialValidationStatus::InvalidApiKey
    } else if message.contains("invalid_signature") {
        LiveCredentialValidationStatus::InvalidSignature
    } else if message.contains("permission_denied") {
        LiveCredentialValidationStatus::PermissionDenied
    } else if message.contains("timestamp_skew") {
        LiveCredentialValidationStatus::TimestampSkew
    } else if message.contains("network_error") {
        LiveCredentialValidationStatus::NetworkError
    } else if message.contains("response_decode_error") {
        LiveCredentialValidationStatus::ResponseDecodeError
    } else {
        LiveCredentialValidationStatus::ExchangeError
    }
}

#[derive(Debug, Deserialize)]
struct BinanceError {
    code: i64,
    msg: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceOrderResponse {
    order_id: i64,
    symbol: String,
    status: String,
    #[serde(default)]
    client_order_id: Option<String>,
    #[serde(default)]
    price: Option<String>,
    #[serde(default)]
    avg_price: Option<String>,
    #[serde(default)]
    orig_qty: Option<String>,
    #[serde(default)]
    executed_qty: Option<String>,
    #[serde(rename = "type")]
    order_type: String,
    side: String,
    #[serde(default)]
    time_in_force: Option<String>,
    #[serde(default)]
    reduce_only: bool,
    #[serde(default)]
    update_time: Option<i64>,
}

impl BinanceOrderResponse {
    fn into_order_record(
        self,
        environment: LiveEnvironment,
        payload: BTreeMap<String, String>,
        fallback_time: i64,
    ) -> LiveOrderRecord {
        let symbol = self.symbol.parse().unwrap_or(Symbol::BtcUsdt);
        let side = parse_live_order_side(&self.side);
        let order_type = parse_live_order_type(&self.order_type);
        let client_order_id = self
            .client_order_id
            .or_else(|| payload.get("newClientOrderId").cloned())
            .unwrap_or_else(|| format!("order_{}", self.order_id));
        let updated_at = self.update_time.unwrap_or(fallback_time);
        LiveOrderRecord {
            id: client_order_id.clone(),
            credential_id: None,
            environment,
            symbol,
            side,
            order_type,
            status: live_order_status_from_binance(&self.status),
            client_order_id,
            exchange_order_id: Some(self.order_id.to_string()),
            quantity: self.orig_qty.unwrap_or_else(|| "0".to_string()),
            price: self.price.filter(|price| price != "0" && price != "0.00"),
            executed_qty: self.executed_qty.unwrap_or_else(|| "0".to_string()),
            avg_price: self
                .avg_price
                .filter(|price| price != "0" && price != "0.00"),
            reduce_only: self.reduce_only
                || payload
                    .get("reduceOnly")
                    .map(|value| value == "true")
                    .unwrap_or(false),
            time_in_force: self.time_in_force,
            intent_id: None,
            intent_hash: None,
            source_signal_id: None,
            reason: "exchange_order_status".to_string(),
            payload,
            last_error: None,
            submitted_at: fallback_time,
            updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceUserTrade {
    symbol: String,
    id: i64,
    order_id: i64,
    price: String,
    qty: String,
    #[serde(default)]
    commission: Option<String>,
    #[serde(default)]
    commission_asset: Option<String>,
    #[serde(default)]
    realized_pnl: Option<String>,
    side: String,
    time: i64,
}

impl BinanceUserTrade {
    fn into_fill_record(self) -> Option<LiveFillRecord> {
        let symbol = self.symbol.parse().ok()?;
        Some(LiveFillRecord {
            id: format!("trade_{}_{}", self.order_id, self.id),
            order_id: None,
            client_order_id: None,
            exchange_order_id: Some(self.order_id.to_string()),
            symbol,
            side: parse_live_order_side(&self.side),
            quantity: self.qty,
            price: self.price,
            commission: self.commission,
            commission_asset: self.commission_asset,
            realized_pnl: self.realized_pnl,
            trade_id: Some(self.id.to_string()),
            event_time: self.time,
            created_at: now_ms(),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListenKeyResponse {
    listen_key: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceAccountResponse {
    #[serde(default)]
    can_trade: bool,
    #[serde(default, deserialize_with = "de_string_f64")]
    total_wallet_balance: f64,
    #[serde(default, deserialize_with = "de_string_f64")]
    total_margin_balance: f64,
    #[serde(default, deserialize_with = "de_string_f64")]
    available_balance: f64,
    #[serde(default)]
    multi_assets_margin: Option<bool>,
    #[serde(default)]
    assets: Vec<BinanceAsset>,
    #[serde(default)]
    positions: Vec<BinancePosition>,
}

impl BinanceAccountResponse {
    fn into_snapshot(self, environment: LiveEnvironment, fetched_at: i64) -> LiveAccountSnapshot {
        LiveAccountSnapshot {
            environment,
            can_trade: self.can_trade,
            multi_assets_margin: self.multi_assets_margin,
            total_wallet_balance: self.total_wallet_balance,
            total_margin_balance: self.total_margin_balance,
            available_balance: self.available_balance,
            assets: self
                .assets
                .into_iter()
                .map(BinanceAsset::into_balance)
                .collect(),
            positions: self
                .positions
                .into_iter()
                .filter_map(BinancePosition::into_snapshot)
                .collect(),
            fetched_at,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceAsset {
    asset: String,
    #[serde(default, deserialize_with = "de_string_f64")]
    wallet_balance: f64,
    #[serde(default, deserialize_with = "de_string_f64")]
    available_balance: f64,
    #[serde(default, deserialize_with = "de_string_f64")]
    unrealized_profit: f64,
}

impl BinanceAsset {
    fn into_balance(self) -> LiveAssetBalance {
        LiveAssetBalance {
            asset: self.asset,
            wallet_balance: self.wallet_balance,
            available_balance: self.available_balance,
            unrealized_pnl: self.unrealized_profit,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinancePosition {
    symbol: String,
    position_side: Option<String>,
    #[serde(default, deserialize_with = "de_string_f64")]
    position_amt: f64,
    #[serde(default, deserialize_with = "de_string_f64")]
    entry_price: f64,
    #[serde(default, deserialize_with = "de_optional_string_f64")]
    mark_price: Option<f64>,
    #[serde(default, deserialize_with = "de_string_f64")]
    unrealized_profit: f64,
    #[serde(default, deserialize_with = "de_optional_string_f64")]
    leverage: Option<f64>,
}

impl BinancePosition {
    fn into_snapshot(self) -> Option<LivePositionSnapshot> {
        let symbol = self.symbol.parse().ok()?;
        Some(LivePositionSnapshot {
            symbol,
            position_side: self.position_side.unwrap_or_else(|| "BOTH".to_string()),
            position_amt: self.position_amt,
            entry_price: self.entry_price,
            mark_price: self.mark_price,
            unrealized_pnl: self.unrealized_profit,
            leverage: self.leverage,
        })
    }
}

#[derive(Debug, Deserialize)]
struct BinanceExchangeInfo {
    symbols: Vec<BinanceExchangeSymbol>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceExchangeSymbol {
    symbol: String,
    status: String,
    base_asset: String,
    quote_asset: String,
    price_precision: i64,
    quantity_precision: i64,
    filters: Vec<BinanceFilter>,
}

impl BinanceExchangeSymbol {
    fn into_rules(
        self,
        environment: LiveEnvironment,
        symbol: Symbol,
        fetched_at: i64,
    ) -> AppResult<LiveSymbolRules> {
        let quote_asset = self
            .quote_asset
            .parse::<QuoteAsset>()
            .map_err(AppError::Validation)?;
        let mut summary = LiveSymbolFilterSummary {
            tick_size: None,
            step_size: None,
            min_qty: None,
            min_notional: None,
        };
        for filter in self.filters {
            match filter.filter_type.as_str() {
                "PRICE_FILTER" => summary.tick_size = filter.tick_size,
                "LOT_SIZE" => {
                    summary.step_size = filter.step_size;
                    summary.min_qty = filter.min_qty;
                }
                "MIN_NOTIONAL" => {
                    summary.min_notional = filter.notional.or(filter.min_notional);
                }
                _ => {}
            }
        }
        Ok(LiveSymbolRules {
            environment,
            symbol,
            status: self.status,
            base_asset: self.base_asset,
            quote_asset,
            price_precision: self.price_precision,
            quantity_precision: self.quantity_precision,
            filters: summary,
            fetched_at,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceFilter {
    #[serde(rename = "filterType")]
    filter_type: String,
    #[serde(default, deserialize_with = "de_optional_string_f64")]
    tick_size: Option<f64>,
    #[serde(default, deserialize_with = "de_optional_string_f64")]
    step_size: Option<f64>,
    #[serde(default, deserialize_with = "de_optional_string_f64")]
    min_qty: Option<f64>,
    #[serde(default, deserialize_with = "de_optional_string_f64")]
    min_notional: Option<f64>,
    #[serde(default, deserialize_with = "de_optional_string_f64")]
    notional: Option<f64>,
}

fn parse_user_data_event(text: &str) -> AppResult<LiveUserDataEvent> {
    let value: serde_json::Value = serde_json::from_str(text)
        .map_err(|error| AppError::Exchange(format!("response_decode_error: {error}")))?;
    let event_type = value
        .get("e")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    match event_type {
        "ACCOUNT_UPDATE" => parse_account_update(value),
        "ORDER_TRADE_UPDATE" => parse_order_trade_update(value),
        "ACCOUNT_CONFIG_UPDATE" => parse_account_config_update(value),
        "listenKeyExpired" => Ok(LiveUserDataEvent::ListenKeyExpired {
            event_time: value
                .get("E")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0),
        }),
        other => Ok(LiveUserDataEvent::Unknown {
            event_type: other.to_string(),
            event_time: value.get("E").and_then(serde_json::Value::as_i64),
        }),
    }
}

fn parse_account_update(value: serde_json::Value) -> AppResult<LiveUserDataEvent> {
    let event_time = value
        .get("E")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);
    let account = value.get("a").cloned().unwrap_or_default();
    let balances = account
        .get("B")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            Some(LiveShadowBalance {
                asset: item.get("a")?.as_str()?.to_string(),
                wallet_balance: string_field(item, "wb").unwrap_or_else(|| "0".to_string()),
                cross_wallet_balance: string_field(item, "cw"),
                balance_change: string_field(item, "bc"),
                updated_at: event_time,
            })
        })
        .collect();
    let positions = account
        .get("P")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            let symbol = item.get("s")?.as_str()?.parse().ok()?;
            Some(LiveShadowPosition {
                symbol,
                position_side: string_field(item, "ps").unwrap_or_else(|| "BOTH".to_string()),
                position_amt: string_field(item, "pa").unwrap_or_else(|| "0".to_string()),
                entry_price: string_field(item, "ep").unwrap_or_else(|| "0".to_string()),
                unrealized_pnl: string_field(item, "up").unwrap_or_else(|| "0".to_string()),
                margin_type: string_field(item, "mt"),
                isolated_wallet: string_field(item, "iw"),
                updated_at: event_time,
            })
        })
        .collect();
    Ok(LiveUserDataEvent::AccountUpdate(LiveAccountShadow {
        environment: LiveEnvironment::Testnet,
        balances,
        positions,
        open_orders: Vec::new(),
        can_trade: true,
        multi_assets_margin: None,
        position_mode: Some("one_way".to_string()),
        last_event_time: Some(event_time),
        last_rest_sync_at: None,
        updated_at: event_time,
        ambiguous: false,
        divergence_reasons: Vec::new(),
    }))
}

fn parse_order_trade_update(value: serde_json::Value) -> AppResult<LiveUserDataEvent> {
    let event_time = value
        .get("E")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);
    let order = value.get("o").cloned().unwrap_or_default();
    let symbol: Symbol = order
        .get("s")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| AppError::Exchange("response_decode_error: missing symbol".to_string()))?
        .parse()
        .map_err(AppError::Validation)?;
    let side = match order
        .get("S")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("BUY")
    {
        "SELL" => LiveOrderSide::Sell,
        _ => LiveOrderSide::Buy,
    };
    let order_type = match order
        .get("o")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("MARKET")
    {
        "LIMIT" => LiveOrderType::Limit,
        _ => LiveOrderType::Market,
    };
    Ok(LiveUserDataEvent::OrderTradeUpdate(Box::new(
        LiveShadowOrder {
            order_id: order
                .get("i")
                .and_then(serde_json::Value::as_i64)
                .map(|id| id.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            client_order_id: string_field(&order, "c"),
            symbol,
            side,
            order_type,
            time_in_force: string_field(&order, "f"),
            original_qty: string_field(&order, "q").unwrap_or_else(|| "0".to_string()),
            executed_qty: string_field(&order, "z").unwrap_or_else(|| "0".to_string()),
            price: string_field(&order, "p"),
            avg_price: string_field(&order, "ap"),
            status: string_field(&order, "X").unwrap_or_else(|| "UNKNOWN".to_string()),
            execution_type: string_field(&order, "x"),
            reduce_only: order
                .get("R")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
            position_side: string_field(&order, "ps"),
            last_filled_qty: string_field(&order, "l"),
            last_filled_price: string_field(&order, "L"),
            commission: string_field(&order, "n"),
            commission_asset: string_field(&order, "N"),
            trade_id: order
                .get("t")
                .and_then(serde_json::Value::as_i64)
                .map(|id| id.to_string()),
            last_update_time: order
                .get("T")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(event_time),
        },
    )))
}

fn parse_account_config_update(value: serde_json::Value) -> AppResult<LiveUserDataEvent> {
    let event_time = value
        .get("E")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);
    let ai = value.get("ai");
    let ac = value.get("ac");
    Ok(LiveUserDataEvent::AccountConfigUpdate {
        event_time,
        position_mode: ai
            .and_then(|item| item.get("j"))
            .and_then(serde_json::Value::as_bool)
            .map(|dual| if dual { "hedge" } else { "one_way" }.to_string()),
        leverage_symbol: ac
            .and_then(|item| item.get("s"))
            .and_then(serde_json::Value::as_str)
            .and_then(|symbol| symbol.parse().ok()),
        leverage: ac
            .and_then(|item| item.get("l"))
            .and_then(serde_json::Value::as_i64),
    })
}

fn string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string)
}

fn parse_live_order_side(value: &str) -> LiveOrderSide {
    match value {
        "SELL" => LiveOrderSide::Sell,
        _ => LiveOrderSide::Buy,
    }
}

fn parse_live_order_type(value: &str) -> LiveOrderType {
    match value {
        "LIMIT" => LiveOrderType::Limit,
        _ => LiveOrderType::Market,
    }
}

fn live_order_status_from_binance(value: &str) -> LiveOrderStatus {
    match value {
        "NEW" => LiveOrderStatus::Working,
        "PARTIALLY_FILLED" => LiveOrderStatus::PartiallyFilled,
        "FILLED" => LiveOrderStatus::Filled,
        "CANCELED" => LiveOrderStatus::Canceled,
        "REJECTED" => LiveOrderStatus::Rejected,
        "EXPIRED" => LiveOrderStatus::Expired,
        _ => LiveOrderStatus::UnknownNeedsRepair,
    }
}

fn de_string_f64<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?.unwrap_or_default();
    if value.is_empty() {
        return Ok(0.0);
    }
    value.parse().map_err(serde::de::Error::custom)
}

fn de_optional_string_f64<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    value
        .filter(|value| !value.is_empty())
        .map(|value| value.parse().map_err(serde::de::Error::custom))
        .transpose()
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::{sign_query, BinanceLiveReadOnly};
    use relxen_app::LiveExchangePort;
    use relxen_domain::{
        LiveCredentialId, LiveCredentialSecret, LiveCredentialValidationStatus, LiveEnvironment,
        LiveOrderStatus, Symbol,
    };

    fn account_body() -> serde_json::Value {
        json!({
            "canTrade": true,
            "totalWalletBalance": "100.0",
            "totalMarginBalance": "100.0",
            "availableBalance": "90.0",
            "multiAssetsMargin": false,
            "assets": [
                {"asset": "USDT", "walletBalance": "100.0", "availableBalance": "90.0", "unrealizedProfit": "0.0"}
            ],
            "positions": [
                {"symbol": "BTCUSDT", "positionSide": "BOTH", "positionAmt": "0.001", "entryPrice": "100000.0", "markPrice": "100100.0", "unrealizedProfit": "0.1", "leverage": "5"}
            ]
        })
    }

    fn exchange_info_body() -> serde_json::Value {
        json!({
            "symbols": [
                {
                    "symbol": "BTCUSDT",
                    "status": "TRADING",
                    "baseAsset": "BTC",
                    "quoteAsset": "USDT",
                    "pricePrecision": 2,
                    "quantityPrecision": 3,
                    "filters": [
                        {"filterType": "PRICE_FILTER", "tickSize": "0.10"},
                        {"filterType": "LOT_SIZE", "stepSize": "0.001", "minQty": "0.001"},
                        {"filterType": "MIN_NOTIONAL", "notional": "100.0"}
                    ]
                }
            ]
        })
    }

    fn listen_key_body() -> serde_json::Value {
        json!({"listenKey": "listen-key-123"})
    }

    fn order_body(status: &str) -> serde_json::Value {
        json!({
            "orderId": 42,
            "symbol": "BTCUSDT",
            "status": status,
            "clientOrderId": "rx_exec_test",
            "price": "0",
            "avgPrice": "0",
            "origQty": "0.001",
            "executedQty": "0",
            "type": "MARKET",
            "side": "BUY",
            "timeInForce": "GTC",
            "reduceOnly": false,
            "updateTime": 100
        })
    }

    fn adapter(server: &MockServer) -> BinanceLiveReadOnly {
        BinanceLiveReadOnly::with_bases(reqwest::Client::new(), server.uri(), server.uri())
    }

    fn secret() -> LiveCredentialSecret {
        LiveCredentialSecret {
            api_key: "api-key".to_string(),
            api_secret: "api-secret".to_string(),
        }
    }

    #[tokio::test]
    async fn validation_uses_signed_account_request_and_api_key_header() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/fapi/v2/account"))
            .and(header("X-MBX-APIKEY", "api-key"))
            .and(query_param("recvWindow", "5000"))
            .respond_with(ResponseTemplate::new(200).set_body_json(account_body()))
            .expect(1)
            .mount(&server)
            .await;

        let result = adapter(&server)
            .validate_credentials(
                LiveEnvironment::Testnet,
                &LiveCredentialId::new("cred-1"),
                &secret(),
            )
            .await
            .unwrap();

        assert_eq!(result.status, LiveCredentialValidationStatus::Valid);
    }

    #[tokio::test]
    async fn validation_maps_invalid_key_failure() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/fapi/v2/account"))
            .respond_with(
                ResponseTemplate::new(401)
                    .set_body_json(json!({"code": -2015, "msg": "Invalid API-key"})),
            )
            .expect(1)
            .mount(&server)
            .await;

        let result = adapter(&server)
            .validate_credentials(
                LiveEnvironment::Mainnet,
                &LiveCredentialId::new("cred-1"),
                &secret(),
            )
            .await
            .unwrap();

        assert_eq!(result.status, LiveCredentialValidationStatus::InvalidApiKey);
    }

    #[tokio::test]
    async fn account_snapshot_parses_balances_and_positions() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/fapi/v2/account"))
            .respond_with(ResponseTemplate::new(200).set_body_json(account_body()))
            .expect(1)
            .mount(&server)
            .await;

        let snapshot = adapter(&server)
            .fetch_account_snapshot(LiveEnvironment::Testnet, &secret())
            .await
            .unwrap();

        assert!(snapshot.can_trade);
        assert_eq!(snapshot.assets[0].asset, "USDT");
        assert_eq!(snapshot.positions[0].symbol, Symbol::BtcUsdt);
    }

    #[tokio::test]
    async fn symbol_rules_parse_required_filters() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/fapi/v1/exchangeInfo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(exchange_info_body()))
            .expect(1)
            .mount(&server)
            .await;

        let rules = adapter(&server)
            .fetch_symbol_rules(LiveEnvironment::Mainnet, Symbol::BtcUsdt)
            .await
            .unwrap();

        assert_eq!(rules.status, "TRADING");
        assert_eq!(rules.filters.tick_size, Some(0.10));
        assert_eq!(rules.filters.step_size, Some(0.001));
        assert_eq!(rules.filters.min_notional, Some(100.0));
    }

    #[tokio::test]
    async fn listen_key_lifecycle_uses_expected_routes_and_headers() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/fapi/v1/listenKey"))
            .and(header("X-MBX-APIKEY", "api-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(listen_key_body()))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("PUT"))
            .and(path("/fapi/v1/listenKey"))
            .and(header("X-MBX-APIKEY", "api-key"))
            .and(query_param("listenKey", "listen-key-123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/fapi/v1/listenKey"))
            .and(header("X-MBX-APIKEY", "api-key"))
            .and(query_param("listenKey", "listen-key-123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(1)
            .mount(&server)
            .await;

        let adapter = adapter(&server);
        let listen_key = adapter
            .create_listen_key(LiveEnvironment::Testnet, &secret())
            .await
            .unwrap();
        assert_eq!(listen_key, "listen-key-123");
        adapter
            .keepalive_listen_key(LiveEnvironment::Testnet, &secret(), &listen_key)
            .await
            .unwrap();
        adapter
            .close_listen_key(LiveEnvironment::Testnet, &secret(), &listen_key)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn order_test_preflight_sends_signed_payload_without_placing_order() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/fapi/v1/order/test"))
            .and(header("X-MBX-APIKEY", "api-key"))
            .and(query_param("symbol", "BTCUSDT"))
            .and(query_param("side", "BUY"))
            .and(query_param("type", "MARKET"))
            .and(query_param("quantity", "0.001"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(1)
            .mount(&server)
            .await;

        let mut payload = std::collections::BTreeMap::new();
        payload.insert("symbol".to_string(), "BTCUSDT".to_string());
        payload.insert("side".to_string(), "BUY".to_string());
        payload.insert("type".to_string(), "MARKET".to_string());
        payload.insert("quantity".to_string(), "0.001".to_string());
        adapter(&server)
            .preflight_order_test(LiveEnvironment::Testnet, &secret(), &payload)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn submit_order_is_testnet_only_and_sends_signed_order_payload() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/fapi/v1/order"))
            .and(header("X-MBX-APIKEY", "api-key"))
            .and(query_param("symbol", "BTCUSDT"))
            .and(query_param("side", "BUY"))
            .and(query_param("type", "MARKET"))
            .and(query_param("quantity", "0.001"))
            .and(query_param("newClientOrderId", "rx_exec_test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(order_body("NEW")))
            .expect(1)
            .mount(&server)
            .await;

        let mut payload = std::collections::BTreeMap::new();
        payload.insert("symbol".to_string(), "BTCUSDT".to_string());
        payload.insert("side".to_string(), "BUY".to_string());
        payload.insert("type".to_string(), "MARKET".to_string());
        payload.insert("quantity".to_string(), "0.001".to_string());
        payload.insert("newClientOrderId".to_string(), "rx_exec_test".to_string());

        let order = adapter(&server)
            .submit_order(LiveEnvironment::Testnet, &secret(), &payload)
            .await
            .unwrap();
        assert_eq!(order.status, LiveOrderStatus::Working);
        assert_eq!(order.client_order_id, "rx_exec_test");

        let error = adapter(&server)
            .submit_order(LiveEnvironment::Mainnet, &secret(), &payload)
            .await
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("execution_not_supported_on_mainnet"));
    }

    #[tokio::test]
    async fn cancel_and_query_order_use_expected_routes() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/fapi/v1/order"))
            .and(header("X-MBX-APIKEY", "api-key"))
            .and(query_param("symbol", "BTCUSDT"))
            .and(query_param("origClientOrderId", "rx_exec_test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(order_body("CANCELED")))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/fapi/v1/order"))
            .and(query_param("symbol", "BTCUSDT"))
            .and(query_param("origClientOrderId", "rx_exec_test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(order_body("FILLED")))
            .expect(1)
            .mount(&server)
            .await;

        let adapter = adapter(&server);
        let canceled = adapter
            .cancel_order(
                LiveEnvironment::Testnet,
                &secret(),
                Symbol::BtcUsdt,
                Some("rx_exec_test"),
                None,
            )
            .await
            .unwrap();
        assert_eq!(canceled.status, LiveOrderStatus::Canceled);

        let queried = adapter
            .query_order(
                LiveEnvironment::Testnet,
                &secret(),
                Symbol::BtcUsdt,
                Some("rx_exec_test"),
                None,
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(queried.status, LiveOrderStatus::Filled);
    }

    #[tokio::test]
    async fn user_trades_parse_into_fill_records() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/fapi/v1/userTrades"))
            .and(query_param("symbol", "BTCUSDT"))
            .and(query_param("limit", "5"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {
                    "symbol": "BTCUSDT",
                    "id": 123,
                    "orderId": 42,
                    "price": "100000",
                    "qty": "0.001",
                    "commission": "0.04",
                    "commissionAsset": "USDT",
                    "realizedPnl": "1.2",
                    "side": "BUY",
                    "time": 100
                }
            ])))
            .expect(1)
            .mount(&server)
            .await;

        let fills = adapter(&server)
            .list_user_trades(LiveEnvironment::Testnet, &secret(), Symbol::BtcUsdt, 5)
            .await
            .unwrap();
        assert_eq!(fills.len(), 1);
        assert_eq!(fills[0].trade_id.as_deref(), Some("123"));
        assert_eq!(fills[0].quantity, "0.001");
    }

    #[test]
    fn user_data_stream_events_parse_account_order_config_and_expiry() {
        let account = super::parse_user_data_event(
            r#"{"e":"ACCOUNT_UPDATE","E":10,"a":{"B":[{"a":"USDT","wb":"1","cw":"1","bc":"0"}],"P":[{"s":"BTCUSDT","pa":"0.001","ep":"100","up":"1","mt":"cross","ps":"BOTH"}]}}"#,
        )
        .unwrap();
        assert!(matches!(
            account,
            relxen_domain::LiveUserDataEvent::AccountUpdate(_)
        ));

        let order = super::parse_user_data_event(
            r#"{"e":"ORDER_TRADE_UPDATE","E":11,"o":{"s":"BTCUSDT","S":"BUY","o":"LIMIT","f":"GTC","q":"0.001","p":"100","ap":"0","x":"NEW","X":"NEW","i":42,"z":"0","R":false,"ps":"BOTH","T":11}}"#,
        )
        .unwrap();
        assert!(matches!(
            order,
            relxen_domain::LiveUserDataEvent::OrderTradeUpdate(_)
        ));

        let config = super::parse_user_data_event(
            r#"{"e":"ACCOUNT_CONFIG_UPDATE","E":12,"ai":{"j":false}}"#,
        )
        .unwrap();
        assert!(matches!(
            config,
            relxen_domain::LiveUserDataEvent::AccountConfigUpdate { .. }
        ));

        let expired = super::parse_user_data_event(r#"{"e":"listenKeyExpired","E":13}"#).unwrap();
        assert!(matches!(
            expired,
            relxen_domain::LiveUserDataEvent::ListenKeyExpired { .. }
        ));
    }

    #[test]
    fn signing_is_deterministic() {
        let signature = sign_query("timestamp=1&recvWindow=5000", "secret").unwrap();
        assert_eq!(signature.len(), 64);
    }
}
