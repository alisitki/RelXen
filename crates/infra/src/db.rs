use std::str::FromStr;
use std::time::Duration;

use anyhow::Context;
use async_trait::async_trait;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Row, SqlitePool};

use relxen_app::{AppError, AppResult, Repository};
use relxen_domain::{
    Candle, LiveAccountShadow, LiveAutoExecutorStatus, LiveCredentialId, LiveCredentialMetadata,
    LiveCredentialSource, LiveCredentialValidationStatus, LiveEnvironment, LiveExecutionSnapshot,
    LiveFillRecord, LiveIntentLock, LiveKillSwitchState, LiveModePreference,
    LiveOrderPreflightResult, LiveOrderRecord, LiveReconciliationStatus, LiveRiskProfile,
    LiveStateRecord, LogEvent, MainnetAutoDecisionEvent, MainnetAutoLessonReport,
    MainnetAutoRiskBudget, MainnetAutoStatus, MainnetAutoWatchdogEvent, Position, PositionSide,
    QuoteAsset, Settings, SignalEvent, SignalSide, Symbol, Timeframe, Trade, TradeAction,
    TradeSource, Wallet,
};

pub struct SqliteRepository {
    pool: SqlitePool,
}

impl SqliteRepository {
    pub async fn connect(database_url: &str) -> AppResult<Self> {
        let options = SqliteConnectOptions::from_str(database_url)
            .context("parsing sqlite connection string")?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .context("connecting sqlite database")?;
        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await
            .context("running sqlite migrations")?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl Repository for SqliteRepository {
    async fn load_settings(&self) -> AppResult<Settings> {
        if let Some(row) = sqlx::query("SELECT * FROM settings WHERE id = 1")
            .fetch_optional(&self.pool)
            .await
            .context("loading settings")?
        {
            let available_symbols = serde_json::from_str::<Vec<Symbol>>(
                row.get::<String, _>("available_symbols_json").as_str(),
            )
            .context("decoding available_symbols_json")?;
            let initial_wallet_balance_by_quote =
                serde_json::from_str(row.get::<String, _>("initial_wallet_balance_json").as_str())
                    .context("decoding initial_wallet_balance_json")?;

            Ok(Settings {
                active_symbol: parse_symbol(row.get("active_symbol"))?,
                available_symbols,
                timeframe: parse_timeframe(row.get("timeframe"))?,
                aso_length: row.get::<i64, _>("aso_length") as usize,
                aso_mode: parse_aso_mode(row.get("aso_mode"))?,
                leverage: row.get("leverage"),
                fee_rate: row.get("fee_rate"),
                sizing_mode: parse_sizing_mode(row.get("sizing_mode"))?,
                fixed_notional: row.get("fixed_notional"),
                initial_wallet_balance_by_quote,
                paper_enabled: row.get::<i64, _>("paper_enabled") != 0,
                live_mode_visible: row.get::<i64, _>("live_mode_visible") != 0,
                auto_restart_on_apply: row.get::<i64, _>("auto_restart_on_apply") != 0,
            })
        } else {
            let settings = Settings::default();
            self.save_settings(&settings).await?;
            Ok(settings)
        }
    }

    async fn save_settings(&self, settings: &Settings) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO settings (
              id, active_symbol, available_symbols_json, timeframe, aso_length, aso_mode,
              leverage, fee_rate, sizing_mode, fixed_notional, initial_wallet_balance_json,
              paper_enabled, live_mode_visible, auto_restart_on_apply, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
              active_symbol = excluded.active_symbol,
              available_symbols_json = excluded.available_symbols_json,
              timeframe = excluded.timeframe,
              aso_length = excluded.aso_length,
              aso_mode = excluded.aso_mode,
              leverage = excluded.leverage,
              fee_rate = excluded.fee_rate,
              sizing_mode = excluded.sizing_mode,
              fixed_notional = excluded.fixed_notional,
              initial_wallet_balance_json = excluded.initial_wallet_balance_json,
              paper_enabled = excluded.paper_enabled,
              live_mode_visible = excluded.live_mode_visible,
              auto_restart_on_apply = excluded.auto_restart_on_apply,
              updated_at = excluded.updated_at
            "#,
        )
        .bind(1_i64)
        .bind(settings.active_symbol.as_str())
        .bind(
            serde_json::to_string(&settings.available_symbols)
                .context("encoding available_symbols_json")?,
        )
        .bind(settings.timeframe.as_str())
        .bind(settings.aso_length as i64)
        .bind(format!("{:?}", settings.aso_mode).to_ascii_lowercase())
        .bind(settings.leverage)
        .bind(settings.fee_rate)
        .bind("fixed_notional")
        .bind(settings.fixed_notional)
        .bind(
            serde_json::to_string(&settings.initial_wallet_balance_by_quote)
                .context("encoding initial_wallet_balance_json")?,
        )
        .bind(bool_to_i64(settings.paper_enabled))
        .bind(bool_to_i64(settings.live_mode_visible))
        .bind(bool_to_i64(settings.auto_restart_on_apply))
        .bind(relxen_app::now_ms())
        .execute(&self.pool)
        .await
        .context("saving settings")?;
        Ok(())
    }

    async fn load_recent_klines(
        &self,
        symbol: Symbol,
        timeframe: Timeframe,
        limit: usize,
    ) -> AppResult<Vec<Candle>> {
        let mut rows = sqlx::query(
            r#"
            SELECT * FROM klines
            WHERE symbol = ? AND timeframe = ?
            ORDER BY open_time DESC
            LIMIT ?
            "#,
        )
        .bind(symbol.as_str())
        .bind(timeframe.as_str())
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .context("loading klines")?;
        rows.reverse();
        rows.into_iter().map(row_to_candle).collect()
    }

    async fn upsert_kline(&self, candle: &Candle) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO klines (
              symbol, timeframe, open_time, close_time, open, high, low, close, volume, closed, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(symbol, timeframe, open_time) DO UPDATE SET
              close_time = excluded.close_time,
              open = excluded.open,
              high = excluded.high,
              low = excluded.low,
              close = excluded.close,
              volume = excluded.volume,
              closed = excluded.closed,
              updated_at = excluded.updated_at
            "#,
        )
        .bind(candle.symbol.as_str())
        .bind(candle.timeframe.as_str())
        .bind(candle.open_time)
        .bind(candle.close_time)
        .bind(candle.open)
        .bind(candle.high)
        .bind(candle.low)
        .bind(candle.close)
        .bind(candle.volume)
        .bind(bool_to_i64(candle.closed))
        .bind(relxen_app::now_ms())
        .execute(&self.pool)
        .await
        .context("upserting kline")?;
        Ok(())
    }

    async fn list_signals(&self, limit: usize) -> AppResult<Vec<SignalEvent>> {
        let mut rows = sqlx::query("SELECT * FROM signals ORDER BY open_time DESC LIMIT ?")
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
            .context("listing signals")?;
        rows.reverse();
        rows.into_iter().map(row_to_signal).collect()
    }

    async fn sync_signals(
        &self,
        symbol: Symbol,
        timeframe: Timeframe,
        signals: &[SignalEvent],
    ) -> AppResult<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("opening signal sync transaction")?;
        sqlx::query("DELETE FROM signals WHERE symbol = ? AND timeframe = ?")
            .bind(symbol.as_str())
            .bind(timeframe.as_str())
            .execute(&mut *tx)
            .await
            .context("clearing signal history for sync")?;
        for signal in signals {
            sqlx::query(
                r#"
                INSERT OR REPLACE INTO signals (
                  id, symbol, timeframe, open_time, side, bulls, bears, closed_only, created_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&signal.id)
            .bind(signal.symbol.as_str())
            .bind(signal.timeframe.as_str())
            .bind(signal.open_time)
            .bind(signal_side_to_string(signal.side))
            .bind(signal.bulls)
            .bind(signal.bears)
            .bind(bool_to_i64(signal.closed_only))
            .bind(relxen_app::now_ms())
            .execute(&mut *tx)
            .await
            .context("inserting signal during sync")?;
        }
        tx.commit()
            .await
            .context("committing signal sync transaction")?;
        Ok(())
    }

    async fn append_signal(&self, signal: &SignalEvent) -> AppResult<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("opening signal append transaction")?;
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO signals (
              id, symbol, timeframe, open_time, side, bulls, bears, closed_only, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&signal.id)
        .bind(signal.symbol.as_str())
        .bind(signal.timeframe.as_str())
        .bind(signal.open_time)
        .bind(signal_side_to_string(signal.side))
        .bind(signal.bulls)
        .bind(signal.bears)
        .bind(bool_to_i64(signal.closed_only))
        .bind(relxen_app::now_ms())
        .execute(&mut *tx)
        .await
        .context("appending signal")?;
        tx.commit().await.context("committing signal append")?;
        Ok(())
    }

    async fn list_trades(&self, limit: usize) -> AppResult<Vec<Trade>> {
        let mut rows = sqlx::query("SELECT * FROM trades ORDER BY timestamp DESC LIMIT ?")
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
            .context("listing trades")?;
        rows.reverse();
        rows.into_iter().map(row_to_trade).collect()
    }

    async fn append_trade(&self, trade: &Trade) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO trades (
              id, symbol, quote_asset, side, action, source, qty, price, notional,
              entry_price, exit_price, fee_paid, realized_pnl, opened_at, closed_at, timestamp
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&trade.id)
        .bind(trade.symbol.as_str())
        .bind(trade.quote_asset.as_str())
        .bind(side_to_string(trade.side))
        .bind(action_to_string(trade.action))
        .bind(trade_source_to_string(trade.source))
        .bind(trade.qty)
        .bind(trade.price)
        .bind(trade.notional)
        .bind(trade.entry_price)
        .bind(trade.exit_price)
        .bind(trade.fee_paid)
        .bind(trade.realized_pnl)
        .bind(trade.opened_at)
        .bind(trade.closed_at)
        .bind(trade.timestamp)
        .execute(&self.pool)
        .await
        .context("appending trade")?;
        Ok(())
    }

    async fn clear_trades(&self) -> AppResult<()> {
        sqlx::query("DELETE FROM trades")
            .execute(&self.pool)
            .await
            .context("clearing trades")?;
        Ok(())
    }

    async fn load_wallets(&self) -> AppResult<Vec<Wallet>> {
        let rows = sqlx::query("SELECT * FROM paper_wallets ORDER BY quote_asset ASC")
            .fetch_all(&self.pool)
            .await
            .context("loading wallets")?;
        rows.into_iter().map(row_to_wallet).collect()
    }

    async fn save_wallets(&self, wallets: &[Wallet]) -> AppResult<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("opening wallet save transaction")?;
        for wallet in wallets {
            sqlx::query(
                r#"
                INSERT INTO paper_wallets (
                  quote_asset, initial_balance, balance, available_balance, reserved_margin,
                  unrealized_pnl, realized_pnl, fees_paid, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(quote_asset) DO UPDATE SET
                  initial_balance = excluded.initial_balance,
                  balance = excluded.balance,
                  available_balance = excluded.available_balance,
                  reserved_margin = excluded.reserved_margin,
                  unrealized_pnl = excluded.unrealized_pnl,
                  realized_pnl = excluded.realized_pnl,
                  fees_paid = excluded.fees_paid,
                  updated_at = excluded.updated_at
                "#,
            )
            .bind(wallet.quote_asset.as_str())
            .bind(wallet.initial_balance)
            .bind(wallet.balance)
            .bind(wallet.available_balance)
            .bind(wallet.reserved_margin)
            .bind(wallet.unrealized_pnl)
            .bind(wallet.realized_pnl)
            .bind(wallet.fees_paid)
            .bind(wallet.updated_at)
            .execute(&mut *tx)
            .await
            .context("saving wallet")?;
        }
        tx.commit().await.context("committing wallet save")?;
        Ok(())
    }

    async fn load_position(&self) -> AppResult<Option<Position>> {
        sqlx::query("SELECT * FROM paper_positions WHERE id = 1")
            .fetch_optional(&self.pool)
            .await
            .context("loading position")?
            .map(row_to_position)
            .transpose()
    }

    async fn save_position(&self, position: Option<&Position>) -> AppResult<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("opening position save transaction")?;
        sqlx::query("DELETE FROM paper_positions WHERE id = 1")
            .execute(&mut *tx)
            .await
            .context("clearing position row")?;

        if let Some(position) = position {
            sqlx::query(
                r#"
                INSERT INTO paper_positions (
                  id, symbol, quote_asset, side, qty, entry_price, mark_price, notional, margin_used,
                  leverage, unrealized_pnl, realized_pnl, opened_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(1_i64)
            .bind(position.symbol.as_str())
            .bind(position.quote_asset.as_str())
            .bind(side_to_string(position.side))
            .bind(position.qty)
            .bind(position.entry_price)
            .bind(position.mark_price)
            .bind(position.notional)
            .bind(position.margin_used)
            .bind(position.leverage)
            .bind(position.unrealized_pnl)
            .bind(position.realized_pnl)
            .bind(position.opened_at)
            .bind(position.updated_at)
            .execute(&mut *tx)
            .await
            .context("inserting position row")?;
        }
        tx.commit().await.context("committing position save")?;
        Ok(())
    }

    async fn recent_logs(&self, limit: usize) -> AppResult<Vec<LogEvent>> {
        let mut rows = sqlx::query("SELECT * FROM logs ORDER BY timestamp DESC LIMIT ?")
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
            .context("loading logs")?;
        rows.reverse();
        rows.into_iter().map(row_to_log).collect()
    }

    async fn append_log(&self, log: &LogEvent) -> AppResult<()> {
        sqlx::query("INSERT OR REPLACE INTO logs (id, timestamp, level, target, message) VALUES (?, ?, ?, ?, ?)")
            .bind(&log.id)
            .bind(log.timestamp)
            .bind(&log.level)
            .bind(&log.target)
            .bind(&log.message)
            .execute(&self.pool)
            .await
            .context("appending log")?;
        Ok(())
    }

    async fn list_live_credentials(&self) -> AppResult<Vec<LiveCredentialMetadata>> {
        let rows = sqlx::query("SELECT * FROM live_credentials ORDER BY created_at ASC")
            .fetch_all(&self.pool)
            .await
            .context("listing live credential metadata")?;
        rows.into_iter().map(row_to_live_credential).collect()
    }

    async fn get_live_credential(
        &self,
        id: &LiveCredentialId,
    ) -> AppResult<Option<LiveCredentialMetadata>> {
        sqlx::query("SELECT * FROM live_credentials WHERE id = ?")
            .bind(id.as_str())
            .fetch_optional(&self.pool)
            .await
            .context("loading live credential metadata")?
            .map(row_to_live_credential)
            .transpose()
    }

    async fn active_live_credential(
        &self,
        environment: LiveEnvironment,
    ) -> AppResult<Option<LiveCredentialMetadata>> {
        sqlx::query(
            "SELECT * FROM live_credentials WHERE environment = ? AND is_active = 1 ORDER BY updated_at DESC LIMIT 1",
        )
        .bind(environment.as_str())
        .fetch_optional(&self.pool)
        .await
        .context("loading active live credential metadata")?
        .map(row_to_live_credential)
        .transpose()
    }

    async fn upsert_live_credential(&self, credential: &LiveCredentialMetadata) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO live_credentials (
              id, alias, environment, source, api_key_hint, validation_status, last_validated_at,
              last_validation_error, is_active, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
              alias = excluded.alias,
              environment = excluded.environment,
              source = excluded.source,
              api_key_hint = excluded.api_key_hint,
              validation_status = excluded.validation_status,
              last_validated_at = excluded.last_validated_at,
              last_validation_error = excluded.last_validation_error,
              is_active = excluded.is_active,
              updated_at = excluded.updated_at
            "#,
        )
        .bind(credential.id.as_str())
        .bind(&credential.alias)
        .bind(credential.environment.as_str())
        .bind(credential.source.as_str())
        .bind(&credential.api_key_hint)
        .bind(credential.validation_status.as_str())
        .bind(credential.last_validated_at)
        .bind(&credential.last_validation_error)
        .bind(bool_to_i64(credential.is_active))
        .bind(credential.created_at)
        .bind(credential.updated_at)
        .execute(&self.pool)
        .await
        .context("upserting live credential metadata")?;
        Ok(())
    }

    async fn delete_live_credential(&self, id: &LiveCredentialId) -> AppResult<()> {
        sqlx::query("DELETE FROM live_credentials WHERE id = ?")
            .bind(id.as_str())
            .execute(&self.pool)
            .await
            .context("deleting live credential metadata")?;
        Ok(())
    }

    async fn select_live_credential(
        &self,
        id: &LiveCredentialId,
        environment: LiveEnvironment,
    ) -> AppResult<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("opening live credential selection transaction")?;
        sqlx::query("UPDATE live_credentials SET is_active = 0 WHERE environment = ?")
            .bind(environment.as_str())
            .execute(&mut *tx)
            .await
            .context("clearing active live credentials")?;
        sqlx::query("UPDATE live_credentials SET is_active = 1, updated_at = ? WHERE id = ?")
            .bind(relxen_app::now_ms())
            .bind(id.as_str())
            .execute(&mut *tx)
            .await
            .context("selecting active live credential")?;
        tx.commit()
            .await
            .context("committing live credential selection")?;
        Ok(())
    }

    async fn load_live_state(&self) -> AppResult<LiveStateRecord> {
        if let Some(row) = sqlx::query("SELECT * FROM live_state WHERE id = 1")
            .fetch_optional(&self.pool)
            .await
            .context("loading live state")?
        {
            Ok(LiveStateRecord {
                mode_preference: parse_live_mode_preference(row.get("mode_preference"))?,
                environment: parse_live_environment(row.get("environment"))?,
                armed: row.get::<i64, _>("armed") != 0,
                updated_at: row.get("updated_at"),
            })
        } else {
            let state = LiveStateRecord {
                updated_at: relxen_app::now_ms(),
                ..LiveStateRecord::default()
            };
            self.save_live_state(&state).await?;
            Ok(state)
        }
    }

    async fn save_live_state(&self, state: &LiveStateRecord) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO live_state (id, mode_preference, environment, armed, updated_at)
            VALUES (1, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
              mode_preference = excluded.mode_preference,
              environment = excluded.environment,
              armed = excluded.armed,
              updated_at = excluded.updated_at
            "#,
        )
        .bind(live_mode_preference_to_string(state.mode_preference))
        .bind(state.environment.as_str())
        .bind(bool_to_i64(state.armed))
        .bind(state.updated_at)
        .execute(&self.pool)
        .await
        .context("saving live state")?;
        Ok(())
    }

    async fn load_live_reconciliation(&self) -> AppResult<Option<LiveReconciliationStatus>> {
        sqlx::query("SELECT reconciliation_json FROM live_shadow_state WHERE id = 1")
            .fetch_optional(&self.pool)
            .await
            .context("loading live reconciliation cache")?
            .map(|row| {
                serde_json::from_str::<LiveReconciliationStatus>(
                    row.get::<String, _>("reconciliation_json").as_str(),
                )
                .context("decoding live reconciliation cache")
                .map_err(AppError::from)
            })
            .transpose()
    }

    async fn save_live_reconciliation(&self, status: &LiveReconciliationStatus) -> AppResult<()> {
        let current_shadow = self.load_live_shadow().await?;
        sqlx::query(
            r#"
            INSERT INTO live_shadow_state (id, reconciliation_json, shadow_json, updated_at)
            VALUES (1, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
              reconciliation_json = excluded.reconciliation_json,
              shadow_json = COALESCE(excluded.shadow_json, live_shadow_state.shadow_json),
              updated_at = excluded.updated_at
            "#,
        )
        .bind(serde_json::to_string(status).context("encoding live reconciliation cache")?)
        .bind(
            current_shadow
                .map(|shadow| serde_json::to_string(&shadow))
                .transpose()
                .context("encoding live shadow cache")?,
        )
        .bind(status.updated_at)
        .execute(&self.pool)
        .await
        .context("saving live reconciliation cache")?;
        Ok(())
    }

    async fn load_live_shadow(&self) -> AppResult<Option<LiveAccountShadow>> {
        sqlx::query("SELECT shadow_json FROM live_shadow_state WHERE id = 1")
            .fetch_optional(&self.pool)
            .await
            .context("loading live shadow cache")?
            .and_then(|row| row.get::<Option<String>, _>("shadow_json"))
            .map(|json| {
                serde_json::from_str::<LiveAccountShadow>(&json)
                    .context("decoding live shadow cache")
                    .map_err(AppError::from)
            })
            .transpose()
    }

    async fn save_live_shadow(&self, shadow: &LiveAccountShadow) -> AppResult<()> {
        let reconciliation =
            self.load_live_reconciliation()
                .await?
                .unwrap_or_else(|| LiveReconciliationStatus {
                    shadow: Some(shadow.clone()),
                    updated_at: shadow.updated_at,
                    ..LiveReconciliationStatus::default()
                });
        sqlx::query(
            r#"
            INSERT INTO live_shadow_state (id, reconciliation_json, shadow_json, updated_at)
            VALUES (1, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
              reconciliation_json = excluded.reconciliation_json,
              shadow_json = excluded.shadow_json,
              updated_at = excluded.updated_at
            "#,
        )
        .bind(
            serde_json::to_string(&LiveReconciliationStatus {
                shadow: Some(shadow.clone()),
                updated_at: shadow.updated_at,
                ..reconciliation
            })
            .context("encoding live reconciliation cache")?,
        )
        .bind(serde_json::to_string(shadow).context("encoding live shadow cache")?)
        .bind(shadow.updated_at)
        .execute(&self.pool)
        .await
        .context("saving live shadow cache")?;
        Ok(())
    }

    async fn list_live_preflights(&self, limit: usize) -> AppResult<Vec<LiveOrderPreflightResult>> {
        let mut rows = sqlx::query(
            "SELECT result_json FROM live_preflight_results ORDER BY created_at DESC LIMIT ?",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .context("listing live preflight results")?;
        rows.reverse();
        rows.into_iter()
            .map(|row| {
                serde_json::from_str::<LiveOrderPreflightResult>(
                    row.get::<String, _>("result_json").as_str(),
                )
                .context("decoding live preflight result")
                .map_err(AppError::from)
            })
            .collect()
    }

    async fn append_live_preflight(&self, result: &LiveOrderPreflightResult) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO live_preflight_results (
              id, environment, symbol, side, order_type, accepted, result_json, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&result.id)
        .bind(result.environment.as_str())
        .bind(result.symbol.as_str())
        .bind(result.side.map(|side| side.as_binance().to_string()))
        .bind(
            result
                .order_type
                .map(|order_type| order_type.as_binance().to_string()),
        )
        .bind(bool_to_i64(result.accepted))
        .bind(serde_json::to_string(result).context("encoding live preflight result")?)
        .bind(result.created_at)
        .execute(&self.pool)
        .await
        .context("appending live preflight result")?;
        Ok(())
    }

    async fn load_live_execution(&self) -> AppResult<Option<LiveExecutionSnapshot>> {
        sqlx::query("SELECT execution_json FROM live_execution_state WHERE id = 1")
            .fetch_optional(&self.pool)
            .await
            .context("loading live execution state")?
            .map(|row| {
                serde_json::from_str::<LiveExecutionSnapshot>(
                    row.get::<String, _>("execution_json").as_str(),
                )
                .context("decoding live execution state")
                .map_err(AppError::from)
            })
            .transpose()
    }

    async fn save_live_execution(&self, execution: &LiveExecutionSnapshot) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO live_execution_state (id, execution_json, updated_at)
            VALUES (1, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
              execution_json = excluded.execution_json,
              updated_at = excluded.updated_at
            "#,
        )
        .bind(serde_json::to_string(execution).context("encoding live execution state")?)
        .bind(execution.updated_at)
        .execute(&self.pool)
        .await
        .context("saving live execution state")?;
        Ok(())
    }

    async fn load_live_kill_switch(&self) -> AppResult<LiveKillSwitchState> {
        load_singleton_json(&self.pool, "live_kill_switch", "state_json")
            .await
            .map(|state| state.unwrap_or_default())
    }

    async fn save_live_kill_switch(&self, state: &LiveKillSwitchState) -> AppResult<()> {
        save_singleton_json(
            &self.pool,
            "live_kill_switch",
            "state_json",
            state,
            state.updated_at,
        )
        .await
    }

    async fn load_live_risk_profile(&self) -> AppResult<LiveRiskProfile> {
        load_singleton_json(&self.pool, "live_risk_profile", "profile_json")
            .await
            .map(|profile| profile.unwrap_or_default())
    }

    async fn save_live_risk_profile(&self, profile: &LiveRiskProfile) -> AppResult<()> {
        save_singleton_json(
            &self.pool,
            "live_risk_profile",
            "profile_json",
            profile,
            profile.updated_at,
        )
        .await
    }

    async fn load_live_auto_executor(&self) -> AppResult<LiveAutoExecutorStatus> {
        load_singleton_json(&self.pool, "live_auto_executor_state", "auto_json")
            .await
            .map(|status| status.unwrap_or_default())
    }

    async fn save_live_auto_executor(&self, status: &LiveAutoExecutorStatus) -> AppResult<()> {
        save_singleton_json(
            &self.pool,
            "live_auto_executor_state",
            "auto_json",
            status,
            status.updated_at,
        )
        .await
    }

    async fn get_live_intent_lock(&self, key: &str) -> AppResult<Option<LiveIntentLock>> {
        sqlx::query("SELECT lock_json FROM live_intent_locks WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .context("loading live intent lock")?
            .map(|row| {
                serde_json::from_str::<LiveIntentLock>(row.get::<String, _>("lock_json").as_str())
                    .context("decoding live intent lock")
                    .map_err(AppError::from)
            })
            .transpose()
    }

    async fn upsert_live_intent_lock(&self, lock: &LiveIntentLock) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO live_intent_locks (
              key, environment, symbol, timeframe, signal_open_time, status,
              order_id, lock_json, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(key) DO UPDATE SET
              status = excluded.status,
              order_id = excluded.order_id,
              lock_json = excluded.lock_json,
              updated_at = excluded.updated_at
            "#,
        )
        .bind(&lock.key)
        .bind(lock.environment.as_str())
        .bind(lock.symbol.as_str())
        .bind(lock.timeframe.as_str())
        .bind(lock.signal_open_time)
        .bind(format!("{:?}", lock.status).to_ascii_lowercase())
        .bind(&lock.order_id)
        .bind(serde_json::to_string(lock).context("encoding live intent lock")?)
        .bind(lock.created_at)
        .bind(lock.updated_at)
        .execute(&self.pool)
        .await
        .context("upserting live intent lock")?;
        Ok(())
    }

    async fn load_mainnet_auto_status(&self) -> AppResult<MainnetAutoStatus> {
        load_singleton_json(&self.pool, "mainnet_auto_state", "status_json")
            .await
            .map(|status| status.unwrap_or_default())
    }

    async fn save_mainnet_auto_status(&self, status: &MainnetAutoStatus) -> AppResult<()> {
        save_singleton_json(
            &self.pool,
            "mainnet_auto_state",
            "status_json",
            status,
            status.updated_at,
        )
        .await
    }

    async fn load_mainnet_auto_risk_budget(&self) -> AppResult<MainnetAutoRiskBudget> {
        load_singleton_json(&self.pool, "mainnet_auto_risk_budget", "budget_json")
            .await
            .map(|budget| budget.unwrap_or_default())
    }

    async fn save_mainnet_auto_risk_budget(&self, budget: &MainnetAutoRiskBudget) -> AppResult<()> {
        save_singleton_json(
            &self.pool,
            "mainnet_auto_risk_budget",
            "budget_json",
            budget,
            budget.updated_at,
        )
        .await
    }

    async fn append_mainnet_auto_decision(
        &self,
        decision: &MainnetAutoDecisionEvent,
    ) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO mainnet_auto_decisions (
              id, session_id, mode, outcome, symbol, timeframe, signal_open_time,
              would_submit, decision_json, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&decision.id)
        .bind(&decision.session_id)
        .bind(format!("{:?}", decision.mode).to_ascii_lowercase())
        .bind(decision.outcome.as_str())
        .bind(decision.symbol.as_str())
        .bind(decision.timeframe.as_str())
        .bind(decision.closed_candle_open_time)
        .bind(if decision.would_submit { 1_i64 } else { 0_i64 })
        .bind(serde_json::to_string(decision).context("encoding mainnet auto decision")?)
        .bind(decision.created_at)
        .execute(&self.pool)
        .await
        .context("appending mainnet auto decision")?;
        Ok(())
    }

    async fn list_mainnet_auto_decisions(
        &self,
        limit: usize,
    ) -> AppResult<Vec<MainnetAutoDecisionEvent>> {
        let mut rows = sqlx::query(
            "SELECT decision_json FROM mainnet_auto_decisions ORDER BY created_at DESC LIMIT ?",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .context("listing mainnet auto decisions")?;
        rows.reverse();
        rows.into_iter()
            .map(|row| {
                serde_json::from_str::<MainnetAutoDecisionEvent>(
                    row.get::<String, _>("decision_json").as_str(),
                )
                .context("decoding mainnet auto decision")
                .map_err(AppError::from)
            })
            .collect()
    }

    async fn append_mainnet_auto_watchdog_event(
        &self,
        event: &MainnetAutoWatchdogEvent,
    ) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO mainnet_auto_watchdog_events (
              id, session_id, reason, event_json, created_at
            ) VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&event.id)
        .bind(&event.session_id)
        .bind(event.reason.as_str())
        .bind(serde_json::to_string(event).context("encoding mainnet auto watchdog event")?)
        .bind(event.created_at)
        .execute(&self.pool)
        .await
        .context("appending mainnet auto watchdog event")?;
        Ok(())
    }

    async fn list_mainnet_auto_watchdog_events(
        &self,
        limit: usize,
    ) -> AppResult<Vec<MainnetAutoWatchdogEvent>> {
        let mut rows = sqlx::query(
            "SELECT event_json FROM mainnet_auto_watchdog_events ORDER BY created_at DESC LIMIT ?",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .context("listing mainnet auto watchdog events")?;
        rows.reverse();
        rows.into_iter()
            .map(|row| {
                serde_json::from_str::<MainnetAutoWatchdogEvent>(
                    row.get::<String, _>("event_json").as_str(),
                )
                .context("decoding mainnet auto watchdog event")
                .map_err(AppError::from)
            })
            .collect()
    }

    async fn save_mainnet_auto_lesson_report(
        &self,
        report: &MainnetAutoLessonReport,
    ) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO mainnet_auto_lesson_reports (
              id, session_id, recommendation, report_json, created_at
            ) VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&report.id)
        .bind(&report.session_id)
        .bind(&report.recommendation)
        .bind(serde_json::to_string(report).context("encoding mainnet auto lesson report")?)
        .bind(report.created_at)
        .execute(&self.pool)
        .await
        .context("saving mainnet auto lesson report")?;
        Ok(())
    }

    async fn latest_mainnet_auto_lesson_report(
        &self,
    ) -> AppResult<Option<MainnetAutoLessonReport>> {
        sqlx::query(
            "SELECT report_json FROM mainnet_auto_lesson_reports ORDER BY created_at DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .context("loading latest mainnet auto lesson report")?
        .map(|row| {
            serde_json::from_str::<MainnetAutoLessonReport>(
                row.get::<String, _>("report_json").as_str(),
            )
            .context("decoding mainnet auto lesson report")
            .map_err(AppError::from)
        })
        .transpose()
    }

    async fn list_live_orders(&self, limit: usize) -> AppResult<Vec<LiveOrderRecord>> {
        let mut rows =
            sqlx::query("SELECT order_json FROM live_orders ORDER BY updated_at DESC LIMIT ?")
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await
                .context("listing live orders")?;
        rows.reverse();
        rows.into_iter()
            .map(|row| {
                serde_json::from_str::<LiveOrderRecord>(row.get::<String, _>("order_json").as_str())
                    .context("decoding live order")
                    .map_err(AppError::from)
            })
            .collect()
    }

    async fn get_live_order(&self, order_ref: &str) -> AppResult<Option<LiveOrderRecord>> {
        sqlx::query(
            r#"
            SELECT order_json FROM live_orders
            WHERE id = ? OR client_order_id = ? OR exchange_order_id = ?
            ORDER BY updated_at DESC LIMIT 1
            "#,
        )
        .bind(order_ref)
        .bind(order_ref)
        .bind(order_ref)
        .fetch_optional(&self.pool)
        .await
        .context("loading live order")?
        .map(|row| {
            serde_json::from_str::<LiveOrderRecord>(row.get::<String, _>("order_json").as_str())
                .context("decoding live order")
                .map_err(AppError::from)
        })
        .transpose()
    }

    async fn upsert_live_order(&self, order: &LiveOrderRecord) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO live_orders (
              id, client_order_id, exchange_order_id, environment, symbol, side,
              order_type, status, order_json, submitted_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
              client_order_id = excluded.client_order_id,
              exchange_order_id = excluded.exchange_order_id,
              environment = excluded.environment,
              symbol = excluded.symbol,
              side = excluded.side,
              order_type = excluded.order_type,
              status = excluded.status,
              order_json = excluded.order_json,
              updated_at = excluded.updated_at
            "#,
        )
        .bind(&order.id)
        .bind(&order.client_order_id)
        .bind(&order.exchange_order_id)
        .bind(order.environment.as_str())
        .bind(order.symbol.as_str())
        .bind(order.side.as_binance())
        .bind(order.order_type.as_binance())
        .bind(order.status.as_str())
        .bind(serde_json::to_string(order).context("encoding live order")?)
        .bind(order.submitted_at)
        .bind(order.updated_at)
        .execute(&self.pool)
        .await
        .context("upserting live order")?;
        Ok(())
    }

    async fn list_live_fills(&self, limit: usize) -> AppResult<Vec<LiveFillRecord>> {
        let mut rows =
            sqlx::query("SELECT fill_json FROM live_fills ORDER BY created_at DESC LIMIT ?")
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await
                .context("listing live fills")?;
        rows.reverse();
        rows.into_iter()
            .map(|row| {
                serde_json::from_str::<LiveFillRecord>(row.get::<String, _>("fill_json").as_str())
                    .context("decoding live fill")
                    .map_err(AppError::from)
            })
            .collect()
    }

    async fn append_live_fill(&self, fill: &LiveFillRecord) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO live_fills (
              id, order_id, client_order_id, exchange_order_id, environment, symbol, side,
              fill_json, event_time, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&fill.id)
        .bind(&fill.order_id)
        .bind(&fill.client_order_id)
        .bind(&fill.exchange_order_id)
        .bind("testnet")
        .bind(fill.symbol.as_str())
        .bind(fill.side.as_binance())
        .bind(serde_json::to_string(fill).context("encoding live fill")?)
        .bind(fill.event_time)
        .bind(fill.created_at)
        .execute(&self.pool)
        .await
        .context("appending live fill")?;
        Ok(())
    }
}

async fn load_singleton_json<T>(
    pool: &SqlitePool,
    table: &str,
    column: &str,
) -> AppResult<Option<T>>
where
    T: for<'de> serde::Deserialize<'de>,
{
    let query = format!("SELECT {column} FROM {table} WHERE id = 1");
    sqlx::query(&query)
        .fetch_optional(pool)
        .await
        .with_context(|| format!("loading singleton json from {table}"))?
        .map(|row| {
            serde_json::from_str::<T>(row.get::<String, _>(column).as_str())
                .with_context(|| format!("decoding singleton json from {table}"))
                .map_err(AppError::from)
        })
        .transpose()
}

async fn save_singleton_json<T>(
    pool: &SqlitePool,
    table: &str,
    column: &str,
    value: &T,
    updated_at: i64,
) -> AppResult<()>
where
    T: serde::Serialize,
{
    let query = format!(
        r#"
        INSERT INTO {table} (id, {column}, updated_at)
        VALUES (1, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
          {column} = excluded.{column},
          updated_at = excluded.updated_at
        "#
    );
    sqlx::query(&query)
        .bind(serde_json::to_string(value).context("encoding singleton json")?)
        .bind(updated_at)
        .execute(pool)
        .await
        .with_context(|| format!("saving singleton json into {table}"))?;
    Ok(())
}

fn row_to_candle(row: sqlx::sqlite::SqliteRow) -> AppResult<Candle> {
    Ok(Candle {
        symbol: parse_symbol(row.get("symbol"))?,
        timeframe: parse_timeframe(row.get("timeframe"))?,
        open_time: row.get("open_time"),
        close_time: row.get("close_time"),
        open: row.get("open"),
        high: row.get("high"),
        low: row.get("low"),
        close: row.get("close"),
        volume: row.get("volume"),
        closed: row.get::<i64, _>("closed") != 0,
    })
}

fn row_to_signal(row: sqlx::sqlite::SqliteRow) -> AppResult<SignalEvent> {
    Ok(SignalEvent {
        id: row.get("id"),
        symbol: parse_symbol(row.get("symbol"))?,
        timeframe: parse_timeframe(row.get("timeframe"))?,
        open_time: row.get("open_time"),
        side: parse_signal_side(row.get("side"))?,
        bulls: row.get("bulls"),
        bears: row.get("bears"),
        closed_only: row.get::<i64, _>("closed_only") != 0,
    })
}

fn row_to_trade(row: sqlx::sqlite::SqliteRow) -> AppResult<Trade> {
    Ok(Trade {
        id: row.get("id"),
        symbol: parse_symbol(row.get("symbol"))?,
        quote_asset: parse_quote_asset(row.get("quote_asset"))?,
        side: parse_position_side(row.get("side"))?,
        action: parse_trade_action(row.get("action"))?,
        source: parse_trade_source(row.get("source"))?,
        qty: row.get("qty"),
        price: row.get("price"),
        notional: row.get("notional"),
        entry_price: row.get("entry_price"),
        exit_price: row.get("exit_price"),
        fee_paid: row.get("fee_paid"),
        realized_pnl: row.get("realized_pnl"),
        opened_at: row.get("opened_at"),
        closed_at: row.get("closed_at"),
        timestamp: row.get("timestamp"),
    })
}

fn row_to_wallet(row: sqlx::sqlite::SqliteRow) -> AppResult<Wallet> {
    Ok(Wallet {
        quote_asset: parse_quote_asset(row.get("quote_asset"))?,
        initial_balance: row.get("initial_balance"),
        balance: row.get("balance"),
        available_balance: row.get("available_balance"),
        reserved_margin: row.get("reserved_margin"),
        unrealized_pnl: row.get("unrealized_pnl"),
        realized_pnl: row.get("realized_pnl"),
        fees_paid: row.get("fees_paid"),
        updated_at: row.get("updated_at"),
    })
}

fn row_to_position(row: sqlx::sqlite::SqliteRow) -> AppResult<Position> {
    Ok(Position {
        symbol: parse_symbol(row.get("symbol"))?,
        quote_asset: parse_quote_asset(row.get("quote_asset"))?,
        side: parse_position_side(row.get("side"))?,
        qty: row.get("qty"),
        entry_price: row.get("entry_price"),
        mark_price: row.get("mark_price"),
        notional: row.get("notional"),
        margin_used: row.get("margin_used"),
        leverage: row.get("leverage"),
        unrealized_pnl: row.get("unrealized_pnl"),
        realized_pnl: row.get("realized_pnl"),
        opened_at: row.get("opened_at"),
        updated_at: row.get("updated_at"),
    })
}

fn row_to_log(row: sqlx::sqlite::SqliteRow) -> AppResult<LogEvent> {
    Ok(LogEvent {
        id: row.get("id"),
        timestamp: row.get("timestamp"),
        level: row.get("level"),
        target: row.get("target"),
        message: row.get("message"),
    })
}

fn row_to_live_credential(row: sqlx::sqlite::SqliteRow) -> AppResult<LiveCredentialMetadata> {
    Ok(LiveCredentialMetadata {
        id: LiveCredentialId::new(row.get::<String, _>("id")),
        alias: row.get("alias"),
        environment: parse_live_environment(row.get("environment"))?,
        source: parse_live_credential_source(row.get("source"))?,
        api_key_hint: row.get("api_key_hint"),
        validation_status: parse_live_validation_status(row.get("validation_status"))?,
        last_validated_at: row.get("last_validated_at"),
        last_validation_error: row.get("last_validation_error"),
        is_active: row.get::<i64, _>("is_active") != 0,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

fn parse_symbol(value: String) -> AppResult<Symbol> {
    value.parse().map_err(AppError::Validation)
}

fn parse_timeframe(value: String) -> AppResult<Timeframe> {
    value.parse().map_err(AppError::Validation)
}

fn parse_quote_asset(value: String) -> AppResult<QuoteAsset> {
    value.parse().map_err(AppError::Validation)
}

fn parse_signal_side(value: String) -> AppResult<SignalSide> {
    match value.as_str() {
        "buy" => Ok(SignalSide::Buy),
        "sell" => Ok(SignalSide::Sell),
        _ => Err(AppError::Validation(format!(
            "invalid signal side: {value}"
        ))),
    }
}

fn parse_position_side(value: String) -> AppResult<PositionSide> {
    match value.as_str() {
        "long" => Ok(PositionSide::Long),
        "short" => Ok(PositionSide::Short),
        _ => Err(AppError::Validation(format!(
            "invalid position side: {value}"
        ))),
    }
}

fn parse_trade_action(value: String) -> AppResult<TradeAction> {
    match value.as_str() {
        "open" => Ok(TradeAction::Open),
        "close" => Ok(TradeAction::Close),
        "reverse" => Ok(TradeAction::Reverse),
        _ => Err(AppError::Validation(format!(
            "invalid trade action: {value}"
        ))),
    }
}

fn parse_trade_source(value: String) -> AppResult<TradeSource> {
    match value.as_str() {
        "signal" => Ok(TradeSource::Signal),
        "manual" => Ok(TradeSource::Manual),
        _ => Err(AppError::Validation(format!(
            "invalid trade source: {value}"
        ))),
    }
}

fn parse_aso_mode(value: String) -> AppResult<relxen_domain::AsoMode> {
    match value.as_str() {
        "intrabar" => Ok(relxen_domain::AsoMode::Intrabar),
        "group" => Ok(relxen_domain::AsoMode::Group),
        "both" => Ok(relxen_domain::AsoMode::Both),
        _ => Err(AppError::Validation(format!("invalid aso_mode: {value}"))),
    }
}

fn parse_sizing_mode(value: String) -> AppResult<relxen_domain::SizingMode> {
    match value.as_str() {
        "fixed_notional" => Ok(relxen_domain::SizingMode::FixedNotional),
        _ => Err(AppError::Validation(format!(
            "invalid sizing_mode: {value}"
        ))),
    }
}

fn parse_live_environment(value: String) -> AppResult<LiveEnvironment> {
    value.parse().map_err(AppError::Validation)
}

fn parse_live_validation_status(value: String) -> AppResult<LiveCredentialValidationStatus> {
    value.parse().map_err(AppError::Validation)
}

fn parse_live_credential_source(value: String) -> AppResult<LiveCredentialSource> {
    value.parse().map_err(AppError::Validation)
}

fn parse_live_mode_preference(value: String) -> AppResult<LiveModePreference> {
    match value.as_str() {
        "paper" => Ok(LiveModePreference::Paper),
        "live_read_only" => Ok(LiveModePreference::LiveReadOnly),
        _ => Err(AppError::Validation(format!(
            "invalid live mode preference: {value}"
        ))),
    }
}

fn bool_to_i64(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn signal_side_to_string(side: SignalSide) -> &'static str {
    match side {
        SignalSide::Buy => "buy",
        SignalSide::Sell => "sell",
    }
}

fn side_to_string(side: PositionSide) -> &'static str {
    match side {
        PositionSide::Long => "long",
        PositionSide::Short => "short",
    }
}

fn action_to_string(action: TradeAction) -> &'static str {
    match action {
        TradeAction::Open => "open",
        TradeAction::Close => "close",
        TradeAction::Reverse => "reverse",
    }
}

fn trade_source_to_string(source: TradeSource) -> &'static str {
    match source {
        TradeSource::Signal => "signal",
        TradeSource::Manual => "manual",
    }
}

fn live_mode_preference_to_string(value: LiveModePreference) -> &'static str {
    match value {
        LiveModePreference::Paper => "paper",
        LiveModePreference::LiveReadOnly => "live_read_only",
    }
}
