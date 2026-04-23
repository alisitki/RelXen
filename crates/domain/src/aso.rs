use std::collections::VecDeque;

use crate::models::{AsoMode, AsoPoint, Candle};

#[derive(Debug, Clone)]
pub struct AsoCalculator {
    length: usize,
    mode: AsoMode,
    recent_candles: VecDeque<Candle>,
    bl_values: VecDeque<f64>,
    br_values: VecDeque<f64>,
}

impl AsoCalculator {
    pub fn new(length: usize, mode: AsoMode) -> Self {
        Self {
            length,
            mode,
            recent_candles: VecDeque::with_capacity(length),
            bl_values: VecDeque::with_capacity(length),
            br_values: VecDeque::with_capacity(length),
        }
    }

    pub fn push_closed(&mut self, candle: Candle) -> AsoPoint {
        let intrabar = intrabar_raw(&candle);
        self.recent_candles.push_back(candle.clone());
        while self.recent_candles.len() > self.length {
            self.recent_candles.pop_front();
        }

        let group = if self.recent_candles.len() == self.length {
            Some(group_raw(&self.recent_candles, &candle))
        } else {
            None
        };

        let raw = match self.mode {
            AsoMode::Intrabar => Some(intrabar),
            AsoMode::Group => group,
            AsoMode::Both => {
                group.map(|group| ((intrabar.0 + group.0) / 2.0, (intrabar.1 + group.1) / 2.0))
            }
        };

        if let Some((bl, br)) = raw {
            self.bl_values.push_back(bl);
            self.br_values.push_back(br);
            while self.bl_values.len() > self.length {
                self.bl_values.pop_front();
                self.br_values.pop_front();
            }
        }

        if self.bl_values.len() == self.length {
            let bulls = self.bl_values.iter().sum::<f64>() / self.length as f64;
            let bears = self.br_values.iter().sum::<f64>() / self.length as f64;

            AsoPoint {
                open_time: candle.open_time,
                bulls: Some(clamp_to_range(bulls)),
                bears: Some(clamp_to_range(bears)),
                length: self.length,
                mode: self.mode,
                ready: true,
            }
        } else {
            AsoPoint {
                open_time: candle.open_time,
                bulls: None,
                bears: None,
                length: self.length,
                mode: self.mode,
                ready: false,
            }
        }
    }
}

pub fn warmup_candles_required(length: usize, mode: AsoMode) -> usize {
    match mode {
        AsoMode::Intrabar => length,
        AsoMode::Group | AsoMode::Both => (length * 2).saturating_sub(1),
    }
}

pub fn compute_aso_series(candles: &[Candle], length: usize, mode: AsoMode) -> Vec<AsoPoint> {
    let mut calculator = AsoCalculator::new(length, mode);
    candles
        .iter()
        .filter(|candle| candle.closed)
        .cloned()
        .map(|candle| calculator.push_closed(candle))
        .collect()
}

fn intrabar_raw(candle: &Candle) -> (f64, f64) {
    let denominator = candle.high - candle.low;
    if denominator.abs() < f64::EPSILON {
        return (50.0, 50.0);
    }

    let bull = 50.0 * ((candle.close - candle.low) + (candle.high - candle.open)) / denominator;
    let bear = 50.0 * ((candle.high - candle.close) + (candle.open - candle.low)) / denominator;
    (clamp_to_range(bull), clamp_to_range(bear))
}

fn group_raw(window: &VecDeque<Candle>, latest: &Candle) -> (f64, f64) {
    let group_open = window
        .front()
        .map(|candle| candle.open)
        .unwrap_or(latest.open);
    let group_high = window
        .iter()
        .map(|candle| candle.high)
        .fold(f64::MIN, f64::max);
    let group_low = window
        .iter()
        .map(|candle| candle.low)
        .fold(f64::MAX, f64::min);
    let denominator = group_high - group_low;

    if denominator.abs() < f64::EPSILON {
        return (50.0, 50.0);
    }

    let bull = 50.0 * ((latest.close - group_low) + (group_high - group_open)) / denominator;
    let bear = 50.0 * ((group_high - latest.close) + (group_open - group_low)) / denominator;
    (clamp_to_range(bull), clamp_to_range(bear))
}

fn clamp_to_range(value: f64) -> f64 {
    if !value.is_finite() {
        return 50.0;
    }

    value.clamp(0.0, 100.0)
}
