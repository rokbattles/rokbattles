use crate::{
    BattleRecord,
    metrics::{casualties, consistency_rate_from_parts, finite_non_negative, trade_ratio},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Metrics {
    pub(crate) damage_per_second: f64,
    pub(crate) sustainability_per_second: f64,
    pub(crate) trade_ratio: f64,
    pub(crate) consistency_rate: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct BattleAggregate {
    sample_count: u64,
    total_duration_seconds: f64,
    inflicted_casualties: f64,
    received_casualties: f64,
    sender_healing: f64,
    decisive_battles: u64,
    wins: u64,
    kill_points: f64,
    opponent_kill_points: f64,
    positive_trades: u64,
}

impl BattleAggregate {
    pub(crate) fn push(&mut self, record: BattleRecord) {
        self.sample_count += record.sample_count;
        self.total_duration_seconds += finite_non_negative(record.total_duration_seconds);
        self.kill_points += finite_non_negative(record.kill_points);
        self.opponent_kill_points += finite_non_negative(record.opponent_kill_points);
        self.inflicted_casualties += casualties(
            record.opponent_dead,
            record.opponent_severely_wounded,
            record.opponent_slightly_wounded,
        );
        self.received_casualties += casualties(
            record.sender_dead,
            record.sender_severely_wounded,
            record.sender_slightly_wounded,
        );
        self.sender_healing += finite_non_negative(record.sender_healing);
        self.decisive_battles += record.decisive_battles;
        self.wins += record.wins.min(record.decisive_battles);
        self.positive_trades += record.positive_trades.min(record.sample_count);
    }

    pub(crate) const fn sample_count(self) -> u64 {
        self.sample_count
    }

    pub(crate) fn metrics(&self) -> Metrics {
        let duration = self.total_duration_seconds.max(1.0);
        let win_rate = if self.decisive_battles == 0 {
            0.0
        } else {
            self.wins as f64 / self.decisive_battles as f64
        };
        let positive_trade_rate = if self.sample_count == 0 {
            0.0
        } else {
            self.positive_trades as f64 / self.sample_count as f64
        };
        let consistency_rate = consistency_rate_from_parts(
            (self.decisive_battles > 0).then_some(win_rate),
            (self.sample_count > 0).then_some(positive_trade_rate),
        )
        .unwrap_or(0.0);

        Metrics {
            damage_per_second: self.inflicted_casualties / duration,
            sustainability_per_second: (self.sender_healing - self.received_casualties) / duration,
            trade_ratio: trade_ratio(self.kill_points, self.opponent_kill_points),
            consistency_rate,
        }
    }
}
