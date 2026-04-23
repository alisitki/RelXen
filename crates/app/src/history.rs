use std::collections::BTreeMap;

use relxen_domain::{warmup_candles_required, AsoMode, Candle, Symbol, Timeframe};

use crate::ports::KlineRangeRequest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryWindow {
    pub start_open_time: i64,
    pub end_open_time: i64,
    pub expected_closed_candles: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryLoadPlan {
    pub symbol: Symbol,
    pub timeframe: Timeframe,
    pub chart_seed_closed_candles: usize,
    pub warmup_closed_candles: usize,
    pub recompute_tail_closed_candles: usize,
    pub requested_closed_candles: usize,
    pub latest_closed_open_time: i64,
    pub window: HistoryWindow,
    pub local_closed_candles: usize,
    pub local_contiguous: bool,
    pub remote_backfill_required: bool,
}

impl HistoryLoadPlan {
    pub fn range_request(self) -> KlineRangeRequest {
        KlineRangeRequest {
            symbol: self.symbol,
            timeframe: self.timeframe,
            start_open_time: self.window.start_open_time,
            end_open_time: self.window.end_open_time,
        }
    }
}

pub fn build_history_load_plan(
    symbol: Symbol,
    timeframe: Timeframe,
    aso_length: usize,
    aso_mode: AsoMode,
    history_limit: usize,
    now_ms: i64,
    local_candles: &[Candle],
) -> HistoryLoadPlan {
    let chart_seed_closed_candles = history_limit.max(1);
    let warmup_closed_candles = warmup_candles_required(aso_length, aso_mode).max(1);
    let recompute_tail_closed_candles = warmup_closed_candles.saturating_sub(1).max(1);
    let requested_closed_candles = chart_seed_closed_candles.max(warmup_closed_candles);
    let latest_closed_open_time = latest_closed_open_time(timeframe, now_ms);
    let window =
        history_window_ending_at(timeframe, latest_closed_open_time, requested_closed_candles);
    let local_window = select_closed_window(local_candles, symbol, timeframe, window);
    let local_contiguous = is_contiguous_closed_window(timeframe, window, &local_window);

    HistoryLoadPlan {
        symbol,
        timeframe,
        chart_seed_closed_candles,
        warmup_closed_candles,
        recompute_tail_closed_candles,
        requested_closed_candles,
        latest_closed_open_time,
        window,
        local_closed_candles: local_window.len(),
        local_contiguous,
        remote_backfill_required: !(local_contiguous
            && local_window.len() == requested_closed_candles),
    }
}

pub fn latest_closed_open_time(timeframe: Timeframe, now_ms: i64) -> i64 {
    timeframe.align_open_time(now_ms - timeframe.duration_ms())
}

pub fn history_window_ending_at(
    timeframe: Timeframe,
    end_open_time: i64,
    expected_closed_candles: usize,
) -> HistoryWindow {
    let clamped_expected_closed_candles = expected_closed_candles.max(1);
    let start_open_time =
        end_open_time - (clamped_expected_closed_candles as i64 - 1) * timeframe.duration_ms();

    HistoryWindow {
        start_open_time,
        end_open_time,
        expected_closed_candles: clamped_expected_closed_candles,
    }
}

pub fn select_closed_window(
    candles: &[Candle],
    symbol: Symbol,
    timeframe: Timeframe,
    window: HistoryWindow,
) -> Vec<Candle> {
    let mut selected: Vec<Candle> = candles
        .iter()
        .filter(|candle| {
            candle.symbol == symbol
                && candle.timeframe == timeframe
                && candle.closed
                && candle.open_time >= window.start_open_time
                && candle.open_time <= window.end_open_time
        })
        .cloned()
        .collect();
    selected.sort_by_key(|candle| candle.open_time);
    selected.dedup_by_key(|candle| candle.open_time);
    selected
}

pub fn is_contiguous_closed_window(
    timeframe: Timeframe,
    window: HistoryWindow,
    candles: &[Candle],
) -> bool {
    validate_closed_window(timeframe, window, candles).is_ok()
}

pub fn validate_closed_window(
    timeframe: Timeframe,
    window: HistoryWindow,
    candles: &[Candle],
) -> Result<(), String> {
    if candles.len() != window.expected_closed_candles {
        return Err(format!(
            "expected {} closed candles but found {}",
            window.expected_closed_candles,
            candles.len()
        ));
    }

    let mut expected_open_time = window.start_open_time;
    for candle in candles {
        if !candle.closed {
            return Err(format!(
                "found unfinished candle at open_time {} inside required history window",
                candle.open_time
            ));
        }
        if candle.open_time != expected_open_time {
            return Err(format!(
                "history window is not contiguous at open_time {}",
                expected_open_time
            ));
        }
        expected_open_time = timeframe.next_open_time(expected_open_time);
    }

    if expected_open_time != timeframe.next_open_time(window.end_open_time) {
        return Err(format!(
            "history window did not finish at expected end_open_time {}",
            window.end_open_time
        ));
    }

    Ok(())
}

pub fn merge_candles(existing: Vec<Candle>, incoming: Vec<Candle>, limit: usize) -> Vec<Candle> {
    let mut merged = BTreeMap::new();
    for candle in existing.into_iter().chain(incoming) {
        merged.insert(candle.open_time, candle);
    }
    let mut values: Vec<Candle> = merged.into_values().collect();
    values.sort_by_key(|candle| candle.open_time);
    if values.len() > limit {
        values = values.split_off(values.len() - limit);
    }
    values
}
