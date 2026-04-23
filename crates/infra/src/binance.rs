use std::sync::Arc;

use anyhow::{anyhow, Context};
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};

use relxen_app::{
    AppError, AppResult, KlineRangeRequest, MarketDataPort, MarketStream, MarketStreamEvent,
};
use relxen_domain::{Candle, Symbol, Timeframe};

const REST_BASE: &str = "https://fapi.binance.com";
const WS_BASE: &str = "wss://fstream.binance.com/ws";
const BINANCE_KLINES_MAX_LIMIT: usize = 1_500;

#[derive(Clone)]
pub struct BinanceMarketData {
    client: Arc<reqwest::Client>,
    rest_base: Arc<str>,
    ws_base: Arc<str>,
}

impl Default for BinanceMarketData {
    fn default() -> Self {
        Self {
            client: Arc::new(reqwest::Client::new()),
            rest_base: Arc::from(REST_BASE),
            ws_base: Arc::from(WS_BASE),
        }
    }
}

impl BinanceMarketData {
    #[cfg(test)]
    fn with_endpoints(
        client: reqwest::Client,
        rest_base: impl Into<String>,
        ws_base: impl Into<String>,
    ) -> Self {
        Self {
            client: Arc::new(client),
            rest_base: Arc::from(rest_base.into()),
            ws_base: Arc::from(ws_base.into()),
        }
    }

    fn rest_klines_url(&self) -> String {
        format!("{}/fapi/v1/klines", self.rest_base.trim_end_matches('/'))
    }
}

#[async_trait]
impl MarketDataPort for BinanceMarketData {
    async fn fetch_klines_range(&self, request: KlineRangeRequest) -> AppResult<Vec<Candle>> {
        if request.end_open_time < request.start_open_time {
            return Ok(Vec::new());
        }

        let aligned_start_open_time = request.timeframe.align_open_time(request.start_open_time);
        let aligned_end_open_time = request.timeframe.align_open_time(request.end_open_time);
        let mut candles = Vec::new();

        for page in build_range_pages(
            request.timeframe,
            aligned_start_open_time,
            aligned_end_open_time,
        ) {
            let mut rows = request_klines_page(
                self.client.as_ref(),
                &self.rest_klines_url(),
                request.symbol,
                request.timeframe,
                page,
            )
            .await?;
            candles.append(&mut rows);
        }

        candles.sort_by_key(|candle| candle.open_time);
        candles.dedup_by_key(|candle| candle.open_time);
        candles.retain(|candle| {
            candle.open_time >= aligned_start_open_time && candle.open_time <= aligned_end_open_time
        });
        Ok(candles)
    }

    async fn subscribe_klines(
        &self,
        symbol: Symbol,
        timeframe: Timeframe,
    ) -> AppResult<MarketStream> {
        let stream_url = format!(
            "{}/{}@kline_{}",
            self.ws_base.trim_end_matches('/'),
            symbol.as_str().to_ascii_lowercase(),
            timeframe.as_str()
        );
        let (ws_stream, _) = tokio_tungstenite::connect_async(stream_url)
            .await
            .context("connecting Binance websocket")?;
        let (mut write, mut read) = ws_stream.split();
        let (tx, rx) = mpsc::channel(256);

        tokio::spawn(async move {
            while let Some(message) = read.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        match serde_json::from_str::<BinanceWsEnvelope>(&text) {
                            Ok(envelope) => {
                                let candle = Candle {
                                    symbol,
                                    timeframe,
                                    open_time: envelope.kline.open_time,
                                    close_time: envelope.kline.close_time,
                                    open: parse_num(&envelope.kline.open),
                                    high: parse_num(&envelope.kline.high),
                                    low: parse_num(&envelope.kline.low),
                                    close: parse_num(&envelope.kline.close),
                                    volume: parse_num(&envelope.kline.volume),
                                    closed: envelope.kline.closed,
                                };
                                if tx
                                    .send(Ok(MarketStreamEvent {
                                        candle,
                                        closed: envelope.kline.closed,
                                    }))
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
                            }
                            Err(error) => {
                                let _ = tx
                                    .send(Err(AppError::Other(
                                        anyhow!(error).context("parsing Binance websocket payload"),
                                    )))
                                    .await;
                                break;
                            }
                        }
                    }
                    Ok(Message::Ping(payload)) => {
                        let _ = write.send(Message::Pong(payload)).await;
                    }
                    Ok(Message::Close(_)) => break,
                    Ok(_) => {}
                    Err(error) => {
                        let _ = tx
                            .send(Err(AppError::Other(
                                anyhow!(error).context("reading Binance websocket message"),
                            )))
                            .await;
                        break;
                    }
                }
            }
            warn!("Binance websocket task exited");
        });

        Ok(Box::pin(ReceiverStream::new(rx)))
    }
}

async fn request_klines_page(
    client: &reqwest::Client,
    rest_url: &str,
    symbol: Symbol,
    timeframe: Timeframe,
    page: RangePage,
) -> AppResult<Vec<Candle>> {
    let query = [
        ("symbol", symbol.as_str().to_string()),
        ("interval", timeframe.as_str().to_string()),
        ("startTime", page.start_time.to_string()),
        ("endTime", page.end_time.to_string()),
        ("limit", page.limit.to_string()),
    ];

    info!(
        event = "adapter_range_request_started",
        symbol = %symbol,
        timeframe = %timeframe,
        start_time = page.start_time,
        end_time = page.end_time,
        limit = page.limit,
        "starting Binance ranged kline request"
    );

    let response = match client.get(rest_url).query(&query).send().await {
        Ok(response) => response,
        Err(error) => {
            warn!(
                event = "adapter_range_request_failed",
                symbol = %symbol,
                timeframe = %timeframe,
                start_time = page.start_time,
                end_time = page.end_time,
                limit = page.limit,
                detail = %error,
                "Binance ranged kline request failed before HTTP response"
            );
            return Err(AppError::Other(
                anyhow!(error).context("requesting Binance ranged kline page"),
            ));
        }
    };

    let status = response.status();
    let body = response
        .text()
        .await
        .context("reading Binance kline response body")?;

    if !status.is_success() {
        let detail = format!("HTTP {status}: {body}");
        warn!(
            event = "adapter_range_request_failed",
            symbol = %symbol,
            timeframe = %timeframe,
            start_time = page.start_time,
            end_time = page.end_time,
            limit = page.limit,
            status = status.as_u16(),
            detail = %detail,
            "Binance ranged kline request returned an error status"
        );
        return Err(AppError::Other(anyhow!(
            "Binance ranged kline request failed: {detail}"
        )));
    }

    let rows: Vec<Vec<serde_json::Value>> = match serde_json::from_str(&body) {
        Ok(rows) => rows,
        Err(error) => {
            warn!(
                event = "adapter_range_request_failed",
                symbol = %symbol,
                timeframe = %timeframe,
                start_time = page.start_time,
                end_time = page.end_time,
                limit = page.limit,
                detail = %error,
                "Binance ranged kline response could not be decoded"
            );
            return Err(AppError::Other(
                anyhow!(error).context("decoding Binance kline response"),
            ));
        }
    };

    let candles = rows
        .into_iter()
        .map(|row| parse_rest_row(symbol, timeframe, row))
        .collect::<Result<Vec<_>, _>>()
        .map_err(AppError::from)?;

    info!(
        event = "adapter_range_request_finished",
        symbol = %symbol,
        timeframe = %timeframe,
        start_time = page.start_time,
        end_time = page.end_time,
        limit = page.limit,
        returned_candles = candles.len(),
        "finished Binance ranged kline request"
    );

    Ok(candles)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RangePage {
    start_time: i64,
    end_time: i64,
    limit: usize,
}

fn build_range_pages(
    timeframe: Timeframe,
    start_open_time: i64,
    end_open_time: i64,
) -> Vec<RangePage> {
    if end_open_time < start_open_time {
        return Vec::new();
    }

    let timeframe_ms = timeframe.duration_ms();
    let mut pages = Vec::new();
    let mut cursor_open_time = timeframe.align_open_time(start_open_time);
    let aligned_end_open_time = timeframe.align_open_time(end_open_time);

    while cursor_open_time <= aligned_end_open_time {
        let remaining = timeframe.count_open_times_between(cursor_open_time, aligned_end_open_time);
        let limit = remaining.min(BINANCE_KLINES_MAX_LIMIT);
        let page_end_open_time = cursor_open_time + (limit as i64 - 1) * timeframe_ms;
        let aligned_page_end_open_time = aligned_end_open_time.min(page_end_open_time);
        pages.push(RangePage {
            start_time: cursor_open_time,
            end_time: timeframe.close_time_for_open(aligned_page_end_open_time),
            limit,
        });
        cursor_open_time = aligned_page_end_open_time + timeframe_ms;
    }

    pages
}

fn parse_rest_row(
    symbol: Symbol,
    timeframe: Timeframe,
    row: Vec<serde_json::Value>,
) -> anyhow::Result<Candle> {
    if row.len() < 7 {
        return Err(anyhow!("unexpected Binance kline row length"));
    }

    Ok(Candle {
        symbol,
        timeframe,
        open_time: row[0]
            .as_i64()
            .ok_or_else(|| anyhow!("missing open_time"))?,
        open: parse_num_value(&row[1])?,
        high: parse_num_value(&row[2])?,
        low: parse_num_value(&row[3])?,
        close: parse_num_value(&row[4])?,
        volume: parse_num_value(&row[5])?,
        close_time: row[6].as_i64().unwrap_or_default(),
        closed: true,
    })
}

fn parse_num(value: &str) -> f64 {
    value.parse::<f64>().unwrap_or(0.0)
}

fn parse_num_value(value: &serde_json::Value) -> anyhow::Result<f64> {
    value
        .as_str()
        .ok_or_else(|| anyhow!("expected Binance numeric string"))?
        .parse::<f64>()
        .context("parsing Binance numeric string")
}

#[derive(Debug, Deserialize)]
struct BinanceWsEnvelope {
    #[serde(rename = "k")]
    kline: BinanceWsKline,
}

#[derive(Debug, Deserialize)]
struct BinanceWsKline {
    #[serde(rename = "t")]
    open_time: i64,
    #[serde(rename = "T")]
    close_time: i64,
    #[serde(rename = "o")]
    open: String,
    #[serde(rename = "c")]
    close: String,
    #[serde(rename = "h")]
    high: String,
    #[serde(rename = "l")]
    low: String,
    #[serde(rename = "v")]
    volume: String,
    #[serde(rename = "x")]
    closed: bool,
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::{build_range_pages, BinanceMarketData, RangePage};
    use relxen_app::{KlineRangeRequest, MarketDataPort};
    use relxen_domain::{Symbol, Timeframe};

    fn market(server: &MockServer) -> BinanceMarketData {
        BinanceMarketData::with_endpoints(reqwest::Client::new(), server.uri(), "ws://127.0.0.1:1")
    }

    fn kline_row(open_time: i64, timeframe: Timeframe) -> serde_json::Value {
        json!([
            open_time,
            "100.0",
            "101.0",
            "99.0",
            "100.5",
            "42.0",
            timeframe.close_time_for_open(open_time)
        ])
    }

    #[test]
    fn range_pages_align_boundaries_to_supported_timeframes() {
        let pages = build_range_pages(Timeframe::M5, 62_000, 1_001_000);

        assert_eq!(
            pages,
            vec![RangePage {
                start_time: 0,
                end_time: 1_199_999,
                limit: 4,
            }]
        );
    }

    #[test]
    fn range_pages_paginate_when_window_exceeds_exchange_limit() {
        let end_open_time = (1_500_i64 + 24) * Timeframe::M1.duration_ms();
        let pages = build_range_pages(Timeframe::M1, 0, end_open_time);

        assert_eq!(pages.len(), 2);
        assert_eq!(
            pages[0],
            RangePage {
                start_time: 0,
                end_time: 1_499 * 60_000 + 59_999,
                limit: 1_500,
            }
        );
        assert_eq!(
            pages[1],
            RangePage {
                start_time: 1_500 * 60_000,
                end_time: (1_524 * 60_000) + 59_999,
                limit: 25,
            }
        );
    }

    #[tokio::test]
    async fn ranged_fetch_requests_single_page_with_explicit_query_params() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/fapi/v1/klines"))
            .and(query_param("symbol", "BTCUSDT"))
            .and(query_param("interval", "1m"))
            .and(query_param("startTime", "0"))
            .and(query_param("endTime", "119999"))
            .and(query_param("limit", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(vec![
                kline_row(0, Timeframe::M1),
                kline_row(60_000, Timeframe::M1),
            ]))
            .expect(1)
            .mount(&server)
            .await;

        let candles = market(&server)
            .fetch_klines_range(KlineRangeRequest {
                symbol: Symbol::BtcUsdt,
                timeframe: Timeframe::M1,
                start_open_time: 0,
                end_open_time: 60_000,
            })
            .await
            .unwrap();

        assert_eq!(candles.len(), 2);
        assert_eq!(candles[0].open_time, 0);
        assert_eq!(candles[1].open_time, 60_000);
    }

    #[tokio::test]
    async fn ranged_fetch_paginates_multi_page_windows_deterministically() {
        let server = MockServer::start().await;
        let first_page = ResponseTemplate::new(200).set_body_json(
            (0..1_500)
                .map(|index| kline_row(index as i64 * 60_000, Timeframe::M1))
                .collect::<Vec<_>>(),
        );
        let second_page = ResponseTemplate::new(200).set_body_json(
            (1_500..1_525)
                .map(|index| kline_row(index as i64 * 60_000, Timeframe::M1))
                .collect::<Vec<_>>(),
        );

        Mock::given(method("GET"))
            .and(path("/fapi/v1/klines"))
            .and(query_param("symbol", "BTCUSDT"))
            .and(query_param("interval", "1m"))
            .and(query_param("startTime", "0"))
            .and(query_param("endTime", "89999999"))
            .and(query_param("limit", "1500"))
            .respond_with(first_page)
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/fapi/v1/klines"))
            .and(query_param("symbol", "BTCUSDT"))
            .and(query_param("interval", "1m"))
            .and(query_param("startTime", "90000000"))
            .and(query_param("endTime", "91499999"))
            .and(query_param("limit", "25"))
            .respond_with(second_page)
            .expect(1)
            .mount(&server)
            .await;

        let candles = market(&server)
            .fetch_klines_range(KlineRangeRequest {
                symbol: Symbol::BtcUsdt,
                timeframe: Timeframe::M1,
                start_open_time: 0,
                end_open_time: 1_524 * 60_000,
            })
            .await
            .unwrap();

        assert_eq!(candles.len(), 1_525);
        assert_eq!(candles.first().unwrap().open_time, 0);
        assert_eq!(candles.last().unwrap().open_time, 1_524 * 60_000);
    }

    #[tokio::test]
    async fn ranged_fetch_aligns_unaligned_boundaries_before_requesting_exchange() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/fapi/v1/klines"))
            .and(query_param("symbol", "BTCUSDT"))
            .and(query_param("interval", "5m"))
            .and(query_param("startTime", "0"))
            .and(query_param("endTime", "1199999"))
            .and(query_param("limit", "4"))
            .respond_with(ResponseTemplate::new(200).set_body_json(vec![
                kline_row(0, Timeframe::M5),
                kline_row(300_000, Timeframe::M5),
                kline_row(600_000, Timeframe::M5),
                kline_row(900_000, Timeframe::M5),
            ]))
            .expect(1)
            .mount(&server)
            .await;

        let candles = market(&server)
            .fetch_klines_range(KlineRangeRequest {
                symbol: Symbol::BtcUsdt,
                timeframe: Timeframe::M5,
                start_open_time: 62_000,
                end_open_time: 1_001_000,
            })
            .await
            .unwrap();

        assert_eq!(candles.len(), 4);
        assert_eq!(candles.first().unwrap().open_time, 0);
        assert_eq!(candles.last().unwrap().open_time, 900_000);
    }

    #[tokio::test]
    async fn ranged_fetch_returns_empty_results_without_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/fapi/v1/klines"))
            .respond_with(ResponseTemplate::new(200).set_body_json(Vec::<serde_json::Value>::new()))
            .expect(1)
            .mount(&server)
            .await;

        let candles = market(&server)
            .fetch_klines_range(KlineRangeRequest {
                symbol: Symbol::BtcUsdt,
                timeframe: Timeframe::M1,
                start_open_time: 0,
                end_open_time: 0,
            })
            .await
            .unwrap();

        assert!(candles.is_empty());
    }

    #[tokio::test]
    async fn ranged_fetch_maps_malformed_payload_failures() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/fapi/v1/klines"))
            .respond_with(ResponseTemplate::new(200).set_body_string("{\"broken\":true}"))
            .expect(1)
            .mount(&server)
            .await;

        let error = market(&server)
            .fetch_klines_range(KlineRangeRequest {
                symbol: Symbol::BtcUsdt,
                timeframe: Timeframe::M1,
                start_open_time: 0,
                end_open_time: 0,
            })
            .await
            .unwrap_err();

        assert!(error
            .to_string()
            .contains("decoding Binance kline response"));
    }

    #[tokio::test]
    async fn ranged_fetch_maps_exchange_client_errors() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/fapi/v1/klines"))
            .respond_with(
                ResponseTemplate::new(400)
                    .set_body_json(json!({ "code": -1100, "msg": "illegal chars" })),
            )
            .expect(1)
            .mount(&server)
            .await;

        let error = market(&server)
            .fetch_klines_range(KlineRangeRequest {
                symbol: Symbol::BtcUsdt,
                timeframe: Timeframe::M1,
                start_open_time: 0,
                end_open_time: 0,
            })
            .await
            .unwrap_err();

        assert!(error.to_string().contains("HTTP 400 Bad Request"));
    }

    #[tokio::test]
    async fn ranged_fetch_maps_exchange_server_errors() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/fapi/v1/klines"))
            .respond_with(ResponseTemplate::new(500).set_body_string("upstream failed"))
            .expect(1)
            .mount(&server)
            .await;

        let error = market(&server)
            .fetch_klines_range(KlineRangeRequest {
                symbol: Symbol::BtcUsdt,
                timeframe: Timeframe::M1,
                start_open_time: 0,
                end_open_time: 0,
            })
            .await
            .unwrap_err();

        assert!(error.to_string().contains("HTTP 500 Internal Server Error"));
    }

    #[tokio::test]
    async fn ranged_fetch_maps_network_failures() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);

        let market = BinanceMarketData::with_endpoints(
            reqwest::Client::new(),
            format!("http://{address}"),
            "ws://127.0.0.1:1",
        );

        let error = market
            .fetch_klines_range(KlineRangeRequest {
                symbol: Symbol::BtcUsdt,
                timeframe: Timeframe::M1,
                start_open_time: 0,
                end_open_time: 0,
            })
            .await
            .unwrap_err();

        assert!(error
            .to_string()
            .contains("requesting Binance ranged kline page"));
    }
}
