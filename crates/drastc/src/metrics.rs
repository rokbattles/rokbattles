use crate::{BattleRecord, MIN_REFERENCE_RANGE};

pub(crate) fn normalized_duration(duration_seconds: f64) -> f64 {
    if duration_seconds.is_finite() && duration_seconds > 0.0 { duration_seconds } else { 1.0 }
}

pub(crate) fn casualties(dead: f64, severely_wounded: f64, slightly_wounded: f64) -> f64 {
    finite_non_negative(dead)
        + finite_non_negative(severely_wounded)
        + finite_non_negative(slightly_wounded)
}

fn lethal_casualties(dead: f64, severely_wounded: f64) -> f64 {
    finite_non_negative(dead) + finite_non_negative(severely_wounded)
}

pub(crate) fn finite_non_negative(value: f64) -> f64 {
    if value.is_finite() && value > 0.0 { value } else { 0.0 }
}

pub(crate) fn is_positive_trade(kill_points: f64, opponent_kill_points: f64) -> bool {
    finite_non_negative(kill_points) > finite_non_negative(opponent_kill_points)
}

pub(crate) fn battle_outcome(record: BattleRecord) -> Option<bool> {
    let inflicted = lethal_casualties(record.opponent_dead, record.opponent_severely_wounded);
    let received = lethal_casualties(record.sender_dead, record.sender_severely_wounded);
    if (inflicted - received).abs() <= MIN_REFERENCE_RANGE {
        None
    } else {
        Some(inflicted > received)
    }
}

pub(crate) fn consistency_rate_from_parts(
    win_rate: Option<f64>,
    positive_trade_rate: Option<f64>,
) -> Option<f64> {
    match (win_rate, positive_trade_rate) {
        (Some(win_rate), Some(positive_trade_rate)) => Some((win_rate + positive_trade_rate) / 2.0),
        (Some(win_rate), None) => Some(win_rate),
        (None, Some(positive_trade_rate)) => Some(positive_trade_rate),
        (None, None) => None,
    }
}

pub(crate) fn trade_ratio(kill_points: f64, opponent_kill_points: f64) -> f64 {
    let kill_points = finite_non_negative(kill_points);
    let opponent_kill_points = finite_non_negative(opponent_kill_points);
    if kill_points <= MIN_REFERENCE_RANGE && opponent_kill_points <= MIN_REFERENCE_RANGE {
        1.0
    } else if opponent_kill_points <= MIN_REFERENCE_RANGE {
        0.0
    } else {
        kill_points / opponent_kill_points
    }
}
