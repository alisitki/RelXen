use uuid::Uuid;

use crate::models::{AsoPoint, SignalEvent, SignalSide, Symbol, Timeframe};

pub fn signal_from_points(
    symbol: Symbol,
    timeframe: Timeframe,
    previous: &AsoPoint,
    current: &AsoPoint,
) -> Option<SignalEvent> {
    if !(previous.ready && current.ready) {
        return None;
    }

    let prev_bulls = previous.bulls?;
    let prev_bears = previous.bears?;
    let bulls = current.bulls?;
    let bears = current.bears?;

    let side = if prev_bulls <= prev_bears && bulls > bears {
        Some(SignalSide::Buy)
    } else if prev_bulls >= prev_bears && bulls < bears {
        Some(SignalSide::Sell)
    } else {
        None
    }?;

    Some(SignalEvent {
        id: Uuid::new_v4().to_string(),
        symbol,
        timeframe,
        open_time: current.open_time,
        side,
        bulls,
        bears,
        closed_only: true,
    })
}

pub fn derive_signal_history(
    symbol: Symbol,
    timeframe: Timeframe,
    points: &[AsoPoint],
) -> Vec<SignalEvent> {
    points
        .windows(2)
        .filter_map(|window| signal_from_points(symbol, timeframe, &window[0], &window[1]))
        .collect()
}
