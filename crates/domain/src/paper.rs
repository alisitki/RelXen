use std::collections::BTreeMap;

use uuid::Uuid;

use crate::models::{
    PerformanceStats, Position, PositionSide, QuoteAsset, Settings, SignalEvent, SignalSide, Trade,
    TradeAction, TradeSource, Wallet,
};

#[derive(Debug, Clone)]
pub struct PaperEngine {
    pub wallets: BTreeMap<QuoteAsset, Wallet>,
    pub position: Option<Position>,
    pub trades: Vec<Trade>,
}

struct OpenPositionRequest {
    quote_asset: QuoteAsset,
    symbol: crate::models::Symbol,
    side: PositionSide,
    price: f64,
    timestamp: i64,
    source: TradeSource,
}

impl PaperEngine {
    pub fn new(settings: &Settings, timestamp: i64) -> Self {
        Self {
            wallets: reset_wallets(&settings.initial_wallet_balance_by_quote, timestamp),
            position: None,
            trades: Vec::new(),
        }
    }

    pub fn with_state(
        wallets: BTreeMap<QuoteAsset, Wallet>,
        position: Option<Position>,
        trades: Vec<Trade>,
    ) -> Self {
        Self {
            wallets,
            position,
            trades,
        }
    }

    pub fn apply_signal(
        &mut self,
        settings: &Settings,
        signal: &SignalEvent,
        price: f64,
        timestamp: i64,
    ) -> Result<(), String> {
        if !settings.paper_enabled {
            return Ok(());
        }

        if let Some(existing) = &self.position {
            match (existing.side, signal.side) {
                (PositionSide::Long, SignalSide::Buy) | (PositionSide::Short, SignalSide::Sell) => {
                    return Ok(());
                }
                _ => {
                    self.close_position(
                        settings.fee_rate,
                        price,
                        timestamp,
                        TradeAction::Reverse,
                        TradeSource::Signal,
                    )?;
                }
            }
        }

        let side = match signal.side {
            SignalSide::Buy => PositionSide::Long,
            SignalSide::Sell => PositionSide::Short,
        };
        self.open_position(
            settings,
            OpenPositionRequest {
                quote_asset: signal.symbol.quote_asset(),
                symbol: signal.symbol,
                side,
                price,
                timestamp,
                source: TradeSource::Signal,
            },
        )
    }

    pub fn close_all(&mut self, fee_rate: f64, price: f64, timestamp: i64) -> Result<(), String> {
        if self.position.is_none() {
            return Ok(());
        }

        self.close_position(
            fee_rate,
            price,
            timestamp,
            TradeAction::Close,
            TradeSource::Manual,
        )
    }

    pub fn reset(&mut self, settings: &Settings, timestamp: i64) {
        self.wallets = reset_wallets(&settings.initial_wallet_balance_by_quote, timestamp);
        self.position = None;
        self.trades.clear();
    }

    fn open_position(
        &mut self,
        settings: &Settings,
        request: OpenPositionRequest,
    ) -> Result<(), String> {
        let wallet = self
            .wallets
            .get_mut(&request.quote_asset)
            .ok_or_else(|| format!("missing wallet for {}", request.quote_asset))?;

        let sizing = open_position_size(
            wallet.available_balance,
            settings.fixed_notional,
            settings.leverage,
            settings.fee_rate,
        );
        if sizing <= 0.0 {
            return Err("insufficient balance for a paper position".to_string());
        }

        let margin = sizing / settings.leverage;
        let entry_fee = sizing * settings.fee_rate;
        let required = margin + entry_fee;
        if required > wallet.available_balance + 1e-9 {
            return Err("insufficient balance for required margin and fee".to_string());
        }

        let qty = sizing / request.price;
        if qty <= 0.0 || !qty.is_finite() {
            return Err("invalid position quantity".to_string());
        }

        wallet.available_balance -= required;
        wallet.balance -= entry_fee;
        wallet.reserved_margin += margin;
        wallet.fees_paid += entry_fee;
        wallet.updated_at = request.timestamp;

        let trade = Trade {
            id: Uuid::new_v4().to_string(),
            symbol: request.symbol,
            quote_asset: request.quote_asset,
            side: request.side,
            action: TradeAction::Open,
            source: request.source,
            qty,
            price: request.price,
            notional: sizing,
            entry_price: Some(request.price),
            exit_price: None,
            fee_paid: entry_fee,
            realized_pnl: 0.0,
            opened_at: Some(request.timestamp),
            closed_at: None,
            timestamp: request.timestamp,
        };
        self.trades.push(trade);

        self.position = Some(Position {
            symbol: request.symbol,
            quote_asset: request.quote_asset,
            side: request.side,
            qty,
            entry_price: request.price,
            mark_price: request.price,
            notional: sizing,
            margin_used: margin,
            leverage: settings.leverage,
            unrealized_pnl: 0.0,
            realized_pnl: 0.0,
            opened_at: request.timestamp,
            updated_at: request.timestamp,
        });
        Ok(())
    }

    fn close_position(
        &mut self,
        fee_rate: f64,
        price: f64,
        timestamp: i64,
        action: TradeAction,
        source: TradeSource,
    ) -> Result<(), String> {
        let position = self
            .position
            .take()
            .ok_or_else(|| "no position to close".to_string())?;
        let wallet = self
            .wallets
            .get_mut(&position.quote_asset)
            .ok_or_else(|| format!("missing wallet for {}", position.quote_asset))?;

        let gross_pnl = pnl(position.side, position.qty, position.entry_price, price);
        let close_notional = position.qty * price;
        let exit_fee = close_notional * fee_rate;
        wallet.available_balance += position.margin_used + gross_pnl - exit_fee;
        wallet.balance += gross_pnl - exit_fee;
        wallet.reserved_margin = (wallet.reserved_margin - position.margin_used).max(0.0);
        wallet.realized_pnl += gross_pnl - exit_fee;
        wallet.unrealized_pnl = 0.0;
        wallet.fees_paid += exit_fee;
        wallet.updated_at = timestamp;

        self.trades.push(Trade {
            id: Uuid::new_v4().to_string(),
            symbol: position.symbol,
            quote_asset: position.quote_asset,
            side: position.side,
            action,
            source,
            qty: position.qty,
            price,
            notional: close_notional,
            entry_price: Some(position.entry_price),
            exit_price: Some(price),
            fee_paid: exit_fee,
            realized_pnl: gross_pnl - exit_fee,
            opened_at: Some(position.opened_at),
            closed_at: Some(timestamp),
            timestamp,
        });

        Ok(())
    }
}

pub fn open_position_size(
    available_balance: f64,
    fixed_notional: f64,
    leverage: f64,
    fee_rate: f64,
) -> f64 {
    if available_balance <= 0.0 || fixed_notional <= 0.0 || leverage <= 0.0 {
        return 0.0;
    }

    let max_affordable = available_balance / ((1.0 / leverage) + fee_rate);
    fixed_notional.min(max_affordable).max(0.0)
}

pub fn reset_wallets(
    balances: &BTreeMap<QuoteAsset, f64>,
    timestamp: i64,
) -> BTreeMap<QuoteAsset, Wallet> {
    balances
        .iter()
        .map(|(quote_asset, balance)| {
            (
                *quote_asset,
                Wallet {
                    quote_asset: *quote_asset,
                    initial_balance: *balance,
                    balance: *balance,
                    available_balance: *balance,
                    reserved_margin: 0.0,
                    unrealized_pnl: 0.0,
                    realized_pnl: 0.0,
                    fees_paid: 0.0,
                    updated_at: timestamp,
                },
            )
        })
        .collect()
}

pub fn mark_to_market(
    wallets: &mut BTreeMap<QuoteAsset, Wallet>,
    position: &mut Option<Position>,
    price: f64,
    timestamp: i64,
) {
    if let Some(position) = position.as_mut() {
        position.mark_price = price;
        position.unrealized_pnl = pnl(position.side, position.qty, position.entry_price, price);
        position.updated_at = timestamp;

        if let Some(wallet) = wallets.get_mut(&position.quote_asset) {
            wallet.unrealized_pnl = position.unrealized_pnl;
            wallet.updated_at = timestamp;
        }
    }
}

pub fn compute_performance(
    wallets: &BTreeMap<QuoteAsset, Wallet>,
    position: &Option<Position>,
    trades: &[Trade],
) -> PerformanceStats {
    let realized_balance = wallets.values().map(|wallet| wallet.balance).sum::<f64>();
    let unrealized_pnl = position.as_ref().map(|p| p.unrealized_pnl).unwrap_or(0.0);
    let initial_balance = wallets
        .values()
        .map(|wallet| wallet.initial_balance)
        .sum::<f64>();
    let equity = realized_balance + unrealized_pnl;
    let realized_pnl = realized_balance - initial_balance;
    let return_pct = if initial_balance.abs() < f64::EPSILON {
        0.0
    } else {
        ((equity - initial_balance) / initial_balance) * 100.0
    };

    let closed: Vec<&Trade> = trades
        .iter()
        .filter(|trade| matches!(trade.action, TradeAction::Close | TradeAction::Reverse))
        .collect();
    let wins = closed
        .iter()
        .filter(|trade| trade.realized_pnl > 0.0)
        .count();

    PerformanceStats {
        realized_pnl,
        unrealized_pnl,
        equity,
        return_pct,
        trades: trades.len(),
        closed_trades: closed.len(),
        win_rate: if closed.is_empty() {
            0.0
        } else {
            (wins as f64 / closed.len() as f64) * 100.0
        },
        fees_paid: wallets.values().map(|wallet| wallet.fees_paid).sum(),
    }
}

fn pnl(side: PositionSide, qty: f64, entry_price: f64, exit_price: f64) -> f64 {
    match side {
        PositionSide::Long => qty * (exit_price - entry_price),
        PositionSide::Short => qty * (entry_price - exit_price),
    }
}
