use anyhow::{Context, Result};
use chrono::{Duration, TimeZone, Utc};
use futures_util::TryStreamExt;
use mongodb::{
    Client, Collection, Database,
    bson::{self, DateTime as BsonDateTime, Document, doc},
    options::FindOptions,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::{debug, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Deserialize)]
struct BattleReportDocument {
    report: TrendReport,
}

#[derive(Debug, Deserialize)]
struct TrendReport {
    #[serde(rename = "self")]
    self_side: TrendParticipant,
    enemy: TrendParticipant,
    battle_results: TrendBattleResults,
}

#[derive(Debug, Deserialize)]
struct TrendParticipant {
    player_id: Option<i64>,
    formation: Option<i32>,
    primary_commander: Option<TrendCommander>,
    secondary_commander: Option<TrendCommander>,
}

#[derive(Debug, Deserialize)]
struct TrendCommander {
    id: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct TrendBattleResults {
    power: Option<i64>,
    enemy_power: Option<i64>,
    kill_score: Option<i64>,
    enemy_kill_score: Option<i64>,
    wounded: Option<i64>,
    severely_wounded: Option<i64>,
    remaining: Option<i64>,
    enemy_remaining: Option<i64>,
}

#[derive(Default)]
struct FormationAccumulator {
    total_battles: u64,
    decided_battles: u64,
    wins: u64,
    unique_pairs: HashSet<(Option<i32>, Option<i32>)>,
    power_delta_sum: f64,
    power_deltas: Vec<f64>,
    kill_point_delta_sum: f64,
    kill_point_deltas: Vec<f64>,
    wounded_sum: f64,
    wounded_values: Vec<f64>,
    severely_wounded_sum: f64,
    severely_wounded_values: Vec<f64>,
}

#[derive(Debug, Serialize)]
struct TrendWindow {
    start: BsonDateTime,
    end: BsonDateTime,
}

#[derive(Debug, Serialize)]
struct QuantileSet {
    count: u64,
    p50: f64,
    p75: f64,
    p99: f64,
}

#[derive(Debug, Serialize, Default)]
struct TrendQuantiles {
    #[serde(skip_serializing_if = "Option::is_none")]
    power_delta: Option<QuantileSet>,
    #[serde(skip_serializing_if = "Option::is_none")]
    kill_point_delta: Option<QuantileSet>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wounded: Option<QuantileSet>,
    #[serde(skip_serializing_if = "Option::is_none")]
    severely_wounded: Option<QuantileSet>,
}

#[derive(Debug, Serialize)]
struct TrendMetrics {
    total_battles: u64,
    decided_battles: u64,
    total_pairs_observed: u64,
    pick_rate: f64,
    win_rate: f64,
    wilson_lower_bound: f64,
    avg_power_delta: f64,
    avg_kill_point_delta: f64,
    avg_wounded: f64,
    avg_severely_wounded: f64,
    quantiles: TrendQuantiles,
}

#[derive(Debug, Serialize)]
struct TrendDocument {
    formation: i32,
    window: TrendWindow,
    generated_at: BsonDateTime,
    totals: TrendTotals,
    metrics: TrendMetrics,
}

#[derive(Debug, Serialize)]
struct TrendTotals {
    total_battles_all_formations: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=info", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let mongo_uri =
        std::env::var("MONGO_URI").context("MONGO_URI environment variable must be set")?;
    let client = Client::with_uri_str(&mongo_uri)
        .await
        .context("failed to create MongoDB client")?;
    let db = client
        .default_database()
        .context("MONGO_URI must include a database name")?;

    let now = Utc::now();
    let window_end_naive = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("valid midnight timestamp");
    let window_end = Utc.from_utc_datetime(&window_end_naive);
    let window_start = window_end - Duration::days(30);

    info!(
        window_start = %window_start,
        window_end = %window_end,
        "starting rolling 30d formation aggregation"
    );

    let stats =
        collect_formation_stats(&db, window_start.timestamp(), window_end.timestamp()).await?;

    if stats.aggregates.is_empty() {
        info!("no qualifying reports found for the rolling window");
        return Ok(());
    }

    persist_trends(&db, &stats, window_start, window_end, now).await?;

    info!(
        formations = stats.aggregates.len(),
        total_battles = stats.total_battles,
        "rolling 30d formation aggregation complete"
    );

    Ok(())
}

struct AggregationResult {
    aggregates: HashMap<i32, FormationAccumulator>,
    total_battles: u64,
}

async fn collect_formation_stats(
    db: &Database,
    window_start: i64,
    window_end: i64,
) -> Result<AggregationResult> {
    let collection: Collection<BattleReportDocument> = db.collection("battleReports");

    let filter = doc! {
        "report.metadata.start_date": { "$gte": window_start, "$lt": window_end },
        "report.enemy.player_id": { "$ne": -2 },
        "report.self.player_id": { "$ne": -2 },
    };

    let find_options = FindOptions::builder().no_cursor_timeout(true).build();
    let mut cursor = collection
        .find(filter)
        .with_options(find_options)
        .await
        .context("failed to query battleReports")?;

    let mut aggregates: HashMap<i32, FormationAccumulator> = HashMap::new();
    let mut processed: u64 = 0;

    while let Some(entry) = cursor
        .try_next()
        .await
        .context("failed to iterate battleReports cursor")?
    {
        processed += 1;

        if entry.report.self_side.player_id == Some(-2) || entry.report.enemy.player_id == Some(-2)
        {
            continue;
        }

        let formation = match entry.report.self_side.formation {
            Some(value) => value,
            None => continue,
        };

        let acc = aggregates.entry(formation).or_default();
        acc.record(&entry.report);
    }

    debug!(
        processed,
        formations = aggregates.len(),
        "processed reports for formations"
    );

    let total_battles = aggregates.values().map(|acc| acc.total_battles).sum();

    Ok(AggregationResult {
        aggregates,
        total_battles,
    })
}

async fn persist_trends(
    db: &Database,
    stats: &AggregationResult,
    window_start: chrono::DateTime<Utc>,
    window_end: chrono::DateTime<Utc>,
    generated_at: chrono::DateTime<Utc>,
) -> Result<()> {
    let collection: Collection<Document> = db.collection("rolling_30d_formations");

    let window_start_bson = BsonDateTime::from_millis(window_start.timestamp_millis());
    let window_end_bson = BsonDateTime::from_millis(window_end.timestamp_millis());
    let generated_at_bson = BsonDateTime::from_millis(generated_at.timestamp_millis());

    for (&formation, acc) in stats.aggregates.iter() {
        let trend_doc = acc.to_trend_document(
            formation,
            stats.total_battles,
            window_start_bson,
            window_end_bson,
            generated_at_bson,
        );

        let filter = doc! {
            "formation": formation,
            "window.end": window_end_bson,
        };

        let serialized =
            bson::to_document(&trend_doc).context("failed to serialize trend document")?;

        collection
            .replace_one(filter, &serialized)
            .upsert(true)
            .await
            .context("failed to upsert trend document")?;
    }

    Ok(())
}

impl FormationAccumulator {
    fn record(&mut self, report: &TrendReport) {
        self.total_battles += 1;

        if let Some(win) = determine_win(&report.battle_results) {
            self.decided_battles += 1;
            if win {
                self.wins += 1;
            }
        }

        let pair = (
            report
                .self_side
                .primary_commander
                .as_ref()
                .and_then(|c| c.id),
            report
                .self_side
                .secondary_commander
                .as_ref()
                .and_then(|c| c.id),
        );

        if pair.0.is_some() || pair.1.is_some() {
            self.unique_pairs.insert(pair);
        }

        if let Some(delta) = compute_power_delta(&report.battle_results) {
            self.power_delta_sum += delta;
            self.power_deltas.push(delta);
        }

        if let Some(delta) = compute_kill_point_delta(&report.battle_results) {
            self.kill_point_delta_sum += delta;
            self.kill_point_deltas.push(delta);
        }

        if let Some(value) = report.battle_results.wounded {
            let value = value as f64;
            self.wounded_sum += value;
            self.wounded_values.push(value);
        }

        if let Some(value) = report.battle_results.severely_wounded {
            let value = value as f64;
            self.severely_wounded_sum += value;
            self.severely_wounded_values.push(value);
        }
    }

    fn to_trend_document(
        &self,
        formation: i32,
        total_battles_all: u64,
        window_start: BsonDateTime,
        window_end: BsonDateTime,
        generated_at: BsonDateTime,
    ) -> TrendDocument {
        let power_delta_count = self.power_deltas.len() as u64;
        let kill_delta_count = self.kill_point_deltas.len() as u64;
        let wounded_count = self.wounded_values.len() as u64;
        let severely_wounded_count = self.severely_wounded_values.len() as u64;

        let avg_power_delta = if power_delta_count > 0 {
            self.power_delta_sum / power_delta_count as f64
        } else {
            0.0
        };
        let avg_kill_point_delta = if kill_delta_count > 0 {
            self.kill_point_delta_sum / kill_delta_count as f64
        } else {
            0.0
        };
        let avg_wounded = if wounded_count > 0 {
            self.wounded_sum / wounded_count as f64
        } else {
            0.0
        };
        let avg_severely_wounded = if severely_wounded_count > 0 {
            self.severely_wounded_sum / severely_wounded_count as f64
        } else {
            0.0
        };

        let quantiles = TrendQuantiles {
            power_delta: QuantileSet::from_values(&self.power_deltas),
            kill_point_delta: QuantileSet::from_values(&self.kill_point_deltas),
            wounded: QuantileSet::from_values(&self.wounded_values),
            severely_wounded: QuantileSet::from_values(&self.severely_wounded_values),
        };

        let pick_rate = if total_battles_all > 0 {
            self.total_battles as f64 / total_battles_all as f64
        } else {
            0.0
        };

        let win_rate = if self.decided_battles > 0 {
            self.wins as f64 / self.decided_battles as f64
        } else {
            0.0
        };

        let wilson_lower_bound = wilson_lower_bound(self.wins, self.decided_battles, 1.96);

        TrendDocument {
            formation,
            window: TrendWindow {
                start: window_start,
                end: window_end,
            },
            generated_at,
            totals: TrendTotals {
                total_battles_all_formations: total_battles_all,
            },
            metrics: TrendMetrics {
                total_battles: self.total_battles,
                decided_battles: self.decided_battles,
                total_pairs_observed: self.unique_pairs.len() as u64,
                pick_rate,
                win_rate,
                wilson_lower_bound,
                avg_power_delta,
                avg_kill_point_delta,
                avg_wounded,
                avg_severely_wounded,
                quantiles,
            },
        }
    }
}

impl QuantileSet {
    fn from_values(values: &[f64]) -> Option<Self> {
        if values.is_empty() {
            return None;
        }

        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        Some(Self {
            count: values.len() as u64,
            p50: percentile(&sorted, 0.5),
            p75: percentile(&sorted, 0.75),
            p99: percentile(&sorted, 0.99),
        })
    }
}

fn percentile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }

    let n = sorted.len();
    if n == 1 {
        return sorted[0];
    }

    let pos = q.clamp(0.0, 1.0) * (n as f64 - 1.0);
    let lower = pos.floor() as usize;
    let upper = pos.ceil() as usize;

    if lower == upper {
        return sorted[lower];
    }

    let weight = pos - lower as f64;
    sorted[lower] + (sorted[upper] - sorted[lower]) * weight
}

fn compute_power_delta(results: &TrendBattleResults) -> Option<f64> {
    Some((results.power? - results.enemy_power?) as f64)
}

fn compute_kill_point_delta(results: &TrendBattleResults) -> Option<f64> {
    Some((results.kill_score? - results.enemy_kill_score?) as f64)
}

fn determine_win(results: &TrendBattleResults) -> Option<bool> {
    if let (Some(our_remaining), Some(enemy_remaining)) =
        (results.remaining, results.enemy_remaining)
        && our_remaining != enemy_remaining
    {
        return Some(our_remaining > enemy_remaining);
    }

    if let (Some(our_kill), Some(enemy_kill)) = (results.kill_score, results.enemy_kill_score)
        && our_kill != enemy_kill
    {
        return Some(our_kill > enemy_kill);
    }

    if let (Some(our_power), Some(enemy_power)) = (results.power, results.enemy_power)
        && our_power != enemy_power
    {
        return Some(our_power > enemy_power);
    }

    None
}

fn wilson_lower_bound(wins: u64, total: u64, z: f64) -> f64 {
    if total == 0 {
        return 0.0;
    }

    let n = total as f64;
    let p = wins as f64 / n;
    let z2 = z * z;
    let denom = 1.0 + z2 / n;
    let centre = p + z2 / (2.0 * n);
    let margin = z * ((p * (1.0 - p) + z2 / (4.0 * n)) / n).sqrt();
    (centre - margin) / denom
}
