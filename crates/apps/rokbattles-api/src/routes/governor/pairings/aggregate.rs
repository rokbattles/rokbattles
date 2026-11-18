use std::collections::{BTreeMap, BTreeSet};

use mongodb::bson::{Bson, Document};
use rokbattles_bson::{nested_array, nested_f64, nested_i64, nested_str};
use rustc_hash::FxHashMap;

use crate::{
    routes::governor::{
        date_range::GovernorDateRange,
        pairings::types::{
            EquipmentToken, LoadoutArmament, LoadoutSnapshot, PairingDurationDistribution,
            PairingPerformanceMetrics, PairingPerformancePoint, PairingPerformanceResponse,
            PairingsRange,
        },
    },
    time_utils::{date_key_utc, normalize_bson_timestamp_millis, normalize_timestamp_millis},
};

#[derive(Debug, Clone)]
pub(super) struct PairingEntry {
    pub(super) self_primary_commander_id: i64,
    pub(super) self_secondary_commander_id: i64,
    pub(super) enemy_primary_commander_id: i64,
    pub(super) enemy_secondary_commander_id: i64,
    pub(super) battle_duration_millis: i64,
    pub(super) delta: BattleTotalsDelta,
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct BattleTotalsDelta {
    pub(super) kill_score: i64,
    pub(super) severely_wounded: i64,
    pub(super) healing_count: i64,
    pub(super) power_loss: i64,
    pub(super) enemy_kill_score: i64,
    pub(super) enemy_severely_wounded: i64,
    pub(super) enemy_wounded: i64,
    pub(super) enemy_power_loss: i64,
}

#[derive(Default)]
struct PerformanceAccumulator {
    metrics: PairingPerformanceMetrics,
    rate_duration_millis: i64,
    damage: i64,
    healing: i64,
    durations: Vec<i64>,
}

impl PerformanceAccumulator {
    fn push(&mut self, entry: &PairingEntry) {
        self.metrics.battles += 1;
        self.metrics.battle_duration_millis += entry.battle_duration_millis;
        if entry.battle_duration_millis > 0 {
            self.durations.push(entry.battle_duration_millis);
        }

        self.metrics.kp_gain += entry.delta.kill_score;
        self.metrics.kp_loss += entry.delta.enemy_kill_score;
        self.metrics.sevs_gain += entry.delta.enemy_severely_wounded;
        self.metrics.sevs_loss += entry.delta.severely_wounded;

        // Reports encode decreases in power as negative deltas.
        self.metrics.power_loss += entry.delta.power_loss.saturating_neg().max(0);
        self.metrics.opponent_power_loss += entry.delta.enemy_power_loss.saturating_neg().max(0);

        // Use Combat Lab's one-second fallback for rates when timing is missing.
        self.rate_duration_millis +=
            if entry.battle_duration_millis > 0 { entry.battle_duration_millis } else { 1_000 };
        self.damage += entry.delta.enemy_wounded + entry.delta.enemy_severely_wounded;
        self.healing += entry.delta.healing_count;
    }

    fn finish(mut self) -> PairingPerformanceMetrics {
        self.metrics.duration_distribution = duration_distribution(&mut self.durations);
        self.metrics.trade_percent = if self.metrics.kp_loss > 0 {
            Some(self.metrics.kp_gain as f64 / self.metrics.kp_loss as f64 * 100.0)
        } else {
            None
        };

        if self.rate_duration_millis > 0 {
            let seconds = self.rate_duration_millis as f64 / 1_000.0;
            self.metrics.dps = Some(self.damage as f64 / seconds);
            self.metrics.sps = Some(self.metrics.sevs_gain as f64 / seconds);
            self.metrics.tps = Some(self.metrics.sevs_loss as f64 / seconds);
            self.metrics.hps = Some(self.healing as f64 / seconds);
        }

        self.metrics
    }
}

fn duration_distribution(durations: &mut [i64]) -> Option<PairingDurationDistribution> {
    if durations.is_empty() {
        return None;
    }

    durations.sort_unstable();
    let percentile = |percent: usize| {
        let rank = (durations.len() - 1) * percent;
        let lower = rank / 100;
        let upper = (lower + 1).min(durations.len() - 1);
        let lower_value = *durations.get(lower)? as f64;
        let upper_value = *durations.get(upper)? as f64;
        Some(lower_value + (upper_value - lower_value) * (rank % 100) as f64 / 100.0)
    };

    Some(PairingDurationDistribution {
        count: durations.len(),
        p10_millis: percentile(10)?,
        p25_millis: percentile(25)?,
        p50_millis: percentile(50)?,
        p75_millis: percentile(75)?,
        p90_millis: percentile(90)?,
    })
}

pub(super) fn aggregate_performance(
    mails: &[Document],
    range: &GovernorDateRange,
    primary: i64,
    secondary: i64,
    loadout_key: Option<&str>,
) -> PairingPerformanceResponse {
    let mut totals = PerformanceAccumulator::default();
    let mut days: BTreeMap<_, _> = (range.start_millis..range.end_millis)
        .step_by(86_400_000)
        .filter_map(|time| date_key_utc(time).map(|date| (date, PerformanceAccumulator::default())))
        .collect();

    for mail in mails {
        if !mail_is_in_range(mail, range)
            || loadout_key
                .is_some_and(|key| build_loadout_key(&build_loadout_snapshot(mail)) != key)
        {
            continue;
        }

        let Some(day) = extract_event_time_millis(mail)
            .and_then(date_key_utc)
            .and_then(|date| days.get_mut(&date))
        else {
            continue;
        };

        for_each_pairing_entry(mail, |entry| {
            if entry.self_primary_commander_id != primary
                || entry.self_secondary_commander_id != secondary
            {
                return;
            }

            for accumulator in [&mut totals, &mut *day] {
                accumulator.push(&entry);
            }
        });
    }

    PairingPerformanceResponse {
        range: PairingsRange { start: range.start.clone(), end: range.end.clone() },
        totals: totals.finish(),
        series: days
            .into_iter()
            .map(|(date, accumulator)| PairingPerformancePoint {
                date,
                metrics: accumulator.finish(),
            })
            .collect(),
    }
}

pub(super) fn mail_is_in_range(mail: &Document, range: &GovernorDateRange) -> bool {
    extract_event_time_millis(mail)
        .map(|event_time| event_time >= range.start_millis && event_time < range.end_millis)
        .unwrap_or(false)
}

fn extract_event_time_millis(mail: &Document) -> Option<i64> {
    normalize_bson_timestamp_millis(mail.get_document("metadata").ok()?.get("mail_time"))
}

pub(super) fn for_each_pairing_entry(mail: &Document, mut push: impl FnMut(PairingEntry)) {
    let self_primary_commander_id =
        nested_i64(mail, &["sender", "commanders", "primary", "id"]).unwrap_or_default();
    let self_secondary_commander_id =
        nested_i64(mail, &["sender", "commanders", "secondary", "id"]).unwrap_or_default();

    let opponents = nested_array(mail, &["opponents"])
        .into_iter()
        .flatten()
        .filter_map(Bson::as_document)
        .filter(|opponent| {
            let player_id = nested_i64(opponent, &["player_id"]).unwrap_or_default();
            player_id > 0
        });

    for opponent in opponents {
        let battle_interval = extract_battle_interval(mail, opponent);
        push(PairingEntry {
            self_primary_commander_id,
            self_secondary_commander_id,
            enemy_primary_commander_id: nested_i64(opponent, &["commanders", "primary", "id"])
                .unwrap_or_default(),
            enemy_secondary_commander_id: nested_i64(opponent, &["commanders", "secondary", "id"])
                .unwrap_or_default(),
            battle_duration_millis: battle_interval.map_or(0, |(start, end)| end - start),
            delta: extract_battle_delta(opponent),
        });
    }
}

fn extract_battle_interval(mail: &Document, opponent: &Document) -> Option<(i64, i64)> {
    let timeline_start = nested_f64(mail, &["timeline", "start_timestamp"]).unwrap_or_default();
    let start_tick = nested_f64(opponent, &["start_tick"]).unwrap_or_default();
    let end_tick = nested_f64(opponent, &["end_tick"]).unwrap_or(start_tick);

    let start = normalize_timestamp_millis(timeline_start + start_tick);
    let end = normalize_timestamp_millis(timeline_start + end_tick);

    match (start, end) {
        (Some(start), Some(end)) if end > start && timeline_start > 0.0 => Some((start, end)),
        _ => None,
    }
}

fn extract_battle_delta(opponent: &Document) -> BattleTotalsDelta {
    let sender =
        |field| nested_i64(opponent, &["battle_results", "sender", field]).unwrap_or_default();
    let enemy =
        |field| nested_i64(opponent, &["battle_results", "opponent", field]).unwrap_or_default();

    BattleTotalsDelta {
        kill_score: sender("kill_points"),
        severely_wounded: sender("severely_wounded"),
        healing_count: sender("heal"),
        power_loss: sender("power"),
        enemy_kill_score: enemy("kill_points"),
        enemy_severely_wounded: enemy("severely_wounded"),
        enemy_wounded: enemy("slightly_wounded"),
        enemy_power_loss: enemy("power"),
    }
}

pub(super) fn build_loadout_snapshot(mail: &Document) -> LoadoutSnapshot {
    let equipment =
        parse_equipment(nested_str(mail, &["sender", "commanders", "primary", "equipment"]));
    let formation = nested_i64(mail, &["sender", "commanders", "primary", "formation"])
        .and_then(|value| (value > 0).then_some(value));

    let mut inscription_ids = BTreeSet::new();
    let mut buff_totals: FxHashMap<i64, f64> = FxHashMap::default();

    for commander_path in [
        ["sender", "commanders", "primary", "armaments"],
        ["sender", "commanders", "secondary", "armaments"],
    ] {
        let armaments =
            nested_array(mail, &commander_path).into_iter().flatten().filter_map(Bson::as_document);

        for armament in armaments {
            if let Some(affix) = nested_str(armament, &["affix"]) {
                for inscription_id in parse_affix_ids(affix) {
                    inscription_ids.insert(inscription_id);
                }
            }

            if let Some(buffs) = nested_str(armament, &["buffs"]) {
                for (buff_id, buff_value) in parse_buff_pairs(buffs) {
                    *buff_totals.entry(buff_id).or_default() += buff_value;
                }
            }
        }
    }

    let inscriptions = inscription_ids.into_iter().collect::<Vec<_>>();
    let mut armament_pairs = buff_totals.into_iter().collect::<Vec<_>>();
    armament_pairs.sort_by_key(|(id, _)| *id);

    LoadoutSnapshot {
        equipment,
        armaments: armament_pairs
            .into_iter()
            .map(|(id, value)| LoadoutArmament { id, value })
            .collect(),
        inscriptions,
        formation,
    }
}

fn parse_equipment(raw: Option<&str>) -> Vec<EquipmentToken> {
    let Some(raw) = raw else {
        return Vec::new();
    };

    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let normalized = trimmed.trim_matches(|ch: char| ch == '{' || ch == '}' || ch.is_whitespace());
    if normalized.is_empty() {
        return Vec::new();
    }

    let mut equipment = normalized
        .split(',')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .filter_map(|token| {
            let mut parts = token.split(':');
            let slot = parts.next()?.trim().parse::<i64>().ok()?;
            let id_craft = parts.next().unwrap_or_default().trim();
            let attr = parts.next().and_then(|value| value.trim().parse::<i64>().ok());

            let mut id_parts = id_craft.split('_');
            let id = id_parts.next()?.trim().parse::<i64>().ok()?;
            let craft = id_parts.next().and_then(|value| value.trim().parse::<i64>().ok());

            Some(EquipmentToken { slot, id, craft, attr })
        })
        .collect::<Vec<_>>();

    equipment.sort_by_key(|token| token.slot);
    equipment
}

fn parse_affix_ids(raw: &str) -> Vec<i64> {
    parse_signed_numbers(raw).into_iter().filter(|id| *id > 0).collect::<Vec<_>>()
}

fn parse_signed_numbers(raw: &str) -> Vec<i64> {
    let mut numbers = Vec::new();
    let mut current = String::new();

    for ch in raw.chars() {
        if ch.is_ascii_digit() || (ch == '-' && current.is_empty()) {
            current.push(ch);
            continue;
        }

        if !current.is_empty() {
            if let Ok(parsed) = current.parse::<i64>() {
                numbers.push(parsed);
            }
            current.clear();
        }
    }

    if !current.is_empty()
        && let Ok(parsed) = current.parse::<i64>()
    {
        numbers.push(parsed);
    }

    numbers
}

fn parse_buff_pairs(raw: &str) -> Vec<(i64, f64)> {
    raw.split(|ch| [',', ';'].contains(&ch))
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .filter_map(|token| {
            let mut parts = token.split(|ch| ['_', ':'].contains(&ch)).map(str::trim);
            let buff_id = parts.next()?.parse::<i64>().ok()?;
            let buff_value = parts.next()?.parse::<f64>().ok()?;
            (buff_id > 0 && buff_value.is_finite()).then_some((buff_id, buff_value))
        })
        .collect::<Vec<_>>()
}

pub(crate) fn build_loadout_key(snapshot: &LoadoutSnapshot) -> String {
    let inscriptions =
        snapshot.inscriptions.iter().map(ToString::to_string).collect::<Vec<_>>().join("|");
    let formation =
        snapshot.formation.map(|value| value.to_string()).unwrap_or_else(|| "none".to_string());

    [
        format!("eq:{}", serialize_equipment(&snapshot.equipment)),
        format!("arm:{}", serialize_armaments(&snapshot.armaments)),
        format!("ins:{inscriptions}"),
        format!("fm:{formation}"),
    ]
    .join("|")
}

fn serialize_equipment(tokens: &[EquipmentToken]) -> String {
    tokens
        .iter()
        .map(|token| {
            let craft = token.craft.unwrap_or_default();
            let attr = token.attr.unwrap_or_default();
            format!("{}:{}_{}:{attr}", token.slot, token.id, craft)
        })
        .collect::<Vec<_>>()
        .join("|")
}

fn serialize_armaments(armaments: &[LoadoutArmament]) -> String {
    armaments
        .iter()
        .map(|buff| format!("{}_{}", buff.id, format_armament_value(buff.value)))
        .collect::<Vec<_>>()
        .join("|")
}

fn format_armament_value(value: f64) -> String {
    let rounded = (value * 1_000_000.0).round() / 1_000_000.0;
    format!("{rounded:.6}").trim_end_matches('0').trim_end_matches('.').to_string()
}

#[cfg(test)]
mod tests {
    use mongodb::bson::doc;

    use super::*;

    fn test_range() -> GovernorDateRange {
        GovernorDateRange {
            start_millis: 1_735_689_600_000,
            end_millis: 1_735_689_600_000 + 10_000,
            start: "2025-01-01".to_string(),
            end: "2025-01-01".to_string(),
        }
    }

    fn build_test_mail(primary_equipment: &str, primary_armament_buffs: &str) -> Document {
        build_test_mail_with_scores(primary_equipment, primary_armament_buffs, 100, 50)
    }

    fn build_test_mail_with_scores(
        primary_equipment: &str,
        primary_armament_buffs: &str,
        kill_score: i64,
        enemy_kill_score: i64,
    ) -> Document {
        doc! {
            "metadata": { "mail_time": 1_735_689_600_000_i64 },
            "timeline": { "start_timestamp": 1_735_689_600_000_i64 },
            "sender": {
                "commanders": {
                    "primary": {
                        "id": 100_i64,
                        "formation": 1_i64,
                        "equipment": primary_equipment,
                        "armaments": [
                            { "affix": "101;-1;202", "buffs": primary_armament_buffs }
                        ]
                    },
                    "secondary": {
                        "id": 200_i64,
                        "armaments": [
                            { "affix": "202;303", "buffs": "2001_10" }
                        ]
                    }
                }
            },
            "opponents": [
                {
                    "player_id": 999_i64,
                    "start_tick": 0_i64,
                    "end_tick": 5_000_i64,
                    "commanders": {
                        "primary": { "id": 300_i64 },
                        "secondary": { "id": 400_i64 }
                    },
                    "battle_results": {
                        "sender": {
                            "kill_points": kill_score,
                            "dead": 5_i64,
                            "severely_wounded": 7_i64,
                            "slightly_wounded": 9_i64,
                            "heal": 25_i64,
                            "power": -300_i64,
                            "attack_power": -100_i64,
                            "skill_power": -200_i64
                        },
                        "opponent": {
                            "kill_points": enemy_kill_score,
                            "dead": 3_i64,
                            "severely_wounded": 11_i64,
                            "slightly_wounded": 13_i64,
                            "power": -600_i64,
                            "attack_power": -250_i64,
                            "skill_power": -350_i64
                        }
                    }
                }
            ]
        }
    }

    #[test]
    fn performance_reconciles_daily_totals_and_weights_trade_by_kp() {
        let mut range = test_range();
        range.end_millis = range.start_millis + 3 * 86_400_000;
        range.end = "2025-01-03".to_string();
        let first = build_test_mail_with_scores("", "", 100, 10);
        let mut second = build_test_mail_with_scores("", "", 100, 190);
        second
            .get_document_mut("metadata")
            .expect("metadata")
            .insert("mail_time", range.start_millis + 86_400_000);
        let response = aggregate_performance(&[first, second], &range, 100, 200, None);
        assert_eq!(
            response.totals,
            PairingPerformanceMetrics {
                battles: 2,
                battle_duration_millis: 10_000,
                duration_distribution: Some(PairingDurationDistribution {
                    count: 2,
                    p10_millis: 5_000.0,
                    p25_millis: 5_000.0,
                    p50_millis: 5_000.0,
                    p75_millis: 5_000.0,
                    p90_millis: 5_000.0,
                }),
                kp_gain: 200,
                kp_loss: 200,
                sevs_gain: 22,
                sevs_loss: 14,
                power_loss: 600,
                opponent_power_loss: 1_200,
                trade_percent: Some(100.0),
                dps: Some(4.8),
                sps: Some(2.2),
                tps: Some(1.4),
                hps: Some(5.0),
            }
        );
        assert_eq!(
            response.series.iter().map(|day| day.metrics.battles).collect::<Vec<_>>(),
            [1, 1, 0]
        );
        assert_eq!(response.series[0].metrics.trade_percent, Some(1_000.0));
        assert_eq!(response.series[2].date, "2025-01-03");
        assert_eq!(response.series[2].metrics, PairingPerformanceMetrics::default());
    }

    #[test]
    fn performance_filters_pairing_loadout_date_and_npc_battles() {
        let selected = build_test_mail("{1:100}", "");
        let key = build_loadout_key(&build_loadout_snapshot(&selected));
        let mut boundary = selected.clone();
        boundary
            .get_document_mut("metadata")
            .expect("metadata")
            .insert("mail_time", test_range().end_millis);
        let mut npc = selected.clone();
        npc.get_array_mut("opponents").expect("opponents")[0]
            .as_document_mut()
            .expect("opponent")
            .insert("player_id", 0);
        let mut other_pairing = selected.clone();
        other_pairing
            .get_document_mut("sender")
            .expect("sender")
            .get_document_mut("commanders")
            .expect("commanders")
            .get_document_mut("secondary")
            .expect("secondary")
            .insert("id", 201);
        let response = aggregate_performance(
            &[selected, build_test_mail("{1:999}", ""), boundary, npc, other_pairing],
            &test_range(),
            100,
            200,
            Some(&key),
        );
        assert_eq!(response.totals.battles, 1);
        assert_eq!(response.totals.kp_gain, 100);
        assert_eq!(response.totals.duration_distribution.expect("one timed battle").count, 1);
    }

    #[test]
    fn performance_keeps_unknown_duration_and_lossless_trade_out_of_finite_values() {
        let mut mail = build_test_mail_with_scores("", "", 100, 0);
        mail.remove("timeline");
        let response = aggregate_performance(&[mail], &test_range(), 100, 200, None);
        assert_eq!(response.totals.battle_duration_millis, 0);
        assert_eq!(response.totals.duration_distribution, None);
        assert_eq!(response.totals.trade_percent, None);
        assert_eq!(response.totals.battles, 1);
        assert_eq!(
            (response.totals.dps, response.totals.sps, response.totals.tps, response.totals.hps),
            (Some(24.0), Some(11.0), Some(7.0), Some(25.0))
        );
    }

    #[test]
    fn duration_percentiles_interpolate_observations_and_exclude_missing_timings() {
        let mut mails: Vec<_> = [1_000, 3_000, 5_000, 7_000, 9_000]
            .into_iter()
            .map(|duration| {
                let mut mail = build_test_mail("", "");
                mail.get_array_mut("opponents").expect("opponents")[0]
                    .as_document_mut()
                    .expect("opponent")
                    .insert("end_tick", duration);
                mail
            })
            .collect();
        let mut missing = build_test_mail("", "");
        missing.remove("timeline");
        mails.push(missing);
        let response = aggregate_performance(&mails, &test_range(), 100, 200, None);
        assert_eq!(response.totals.battles, 6);
        assert_eq!(response.totals.battle_duration_millis, 25_000);
        assert_eq!(
            response.totals.duration_distribution,
            Some(PairingDurationDistribution {
                count: 5,
                p10_millis: 1_800.0,
                p25_millis: 3_000.0,
                p50_millis: 5_000.0,
                p75_millis: 7_000.0,
                p90_millis: 8_200.0,
            })
        );
    }

    #[test]
    fn overall_duration_percentiles_use_all_battles_instead_of_averaging_daily_percentiles() {
        let mut range = test_range();
        range.end_millis = range.start_millis + 2 * 86_400_000;
        range.end = "2025-01-02".to_string();
        let mut first = build_test_mail("", "");
        first.get_array_mut("opponents").expect("opponents")[0]
            .as_document_mut()
            .expect("opponent")
            .insert("end_tick", 1_000);
        let mut second = build_test_mail("", "");
        second
            .get_document_mut("metadata")
            .expect("metadata")
            .insert("mail_time", range.start_millis + 86_400_000);
        second.get_array_mut("opponents").expect("opponents")[0]
            .as_document_mut()
            .expect("opponent")
            .insert("end_tick", 10_000);
        let response = aggregate_performance(
            &[first, second.clone(), second.clone(), second],
            &range,
            100,
            200,
            None,
        );
        assert_eq!(
            response.totals.duration_distribution.expect("all timed battles").p50_millis,
            10_000.0
        );
        assert_eq!(
            response.series[0].metrics.duration_distribution.as_ref().expect("day one").p50_millis,
            1_000.0
        );
        assert_eq!(
            response.series[1].metrics.duration_distribution.as_ref().expect("day two").count,
            3
        );
    }

    #[test]
    fn tempo_rates_use_combined_duration_instead_of_averaging_battle_rates() {
        let first = build_test_mail("", "");
        let mut second = first.clone();
        second.get_array_mut("opponents").expect("opponents")[0]
            .as_document_mut()
            .expect("opponent")
            .insert("end_tick", 15_000);
        let response = aggregate_performance(&[first, second], &test_range(), 100, 200, None);
        assert_eq!(
            (response.totals.dps, response.totals.sps, response.totals.tps, response.totals.hps),
            (Some(2.4), Some(1.1), Some(0.7), Some(2.5))
        );
    }

    #[test]
    fn performance_does_not_count_power_increases_as_losses() {
        let mut mail = build_test_mail("", "");
        let results = mail.get_array_mut("opponents").expect("opponents")[0]
            .as_document_mut()
            .expect("opponent")
            .get_document_mut("battle_results")
            .expect("results");
        results.get_document_mut("sender").expect("sender").insert("power", 10);
        let response = aggregate_performance(&[mail], &test_range(), 100, 200, None);
        assert_eq!((response.totals.power_loss, response.totals.opponent_power_loss), (0, 600));
    }

    #[test]
    fn aggregate_loadouts_preserves_equipment_attributes() {
        let mails = vec![
            build_test_mail("{1:100_2:21}", "1000_2;1001_3"),
            build_test_mail("{1:100_2:29}", "1000_2;1001_3"),
        ];

        let items =
            super::super::detail::aggregate_detail(&mails, &test_range(), 100, 200).loadouts;
        assert_eq!(items.len(), 2);
        let attrs =
            items.iter().map(|item| item.loadout.equipment[0].attr).collect::<BTreeSet<_>>();
        assert_eq!(attrs, BTreeSet::from([Some(21), Some(29)]));
        assert!(items.iter().all(|item| item.stats.battles == 1));
    }

    #[test]
    fn aggregate_loadouts_preserves_armament_buff_differences() {
        let mails = vec![
            build_test_mail("{1:100_2:21}", "1000_2;1001_3"),
            build_test_mail("{1:100_2:21}", "1000_9;1001_3"),
        ];

        let items =
            super::super::detail::aggregate_detail(&mails, &test_range(), 100, 200).loadouts;
        assert_eq!(items.len(), 2);
        assert_ne!(items[0].key, items[1].key);
        assert!(items.iter().all(|item| item.stats.battles == 1));
    }

    #[test]
    fn aggregate_loadouts_groups_matching_details_and_preserves_key_format() {
        let mails = vec![
            build_test_mail("{1:100_2:25}", "1000_2;2001_3"),
            build_test_mail("{1:100_2:25}", "2001_3;1000_2"),
        ];

        let items =
            super::super::detail::aggregate_detail(&mails, &test_range(), 100, 200).loadouts;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].stats.battles, 2);
        assert_eq!(items[0].stats.kp_gain, 200);
        assert_eq!(items[0].key, "eq:1:100_2:25|arm:1000_2|2001_13|ins:101|202|303|fm:1");
        assert_eq!(items[0].loadout.inscriptions, vec![101, 202, 303]);
        assert_eq!(items[0].loadout.formation, Some(1));
    }

    #[test]
    fn aggregate_loadouts_parses_decimal_armament_buff_values() {
        let mails =
            vec![build_test_mail("{1:100_2:21}", "3001_0.026000;4001_0.015000;5001_0.022000")];

        let items =
            super::super::detail::aggregate_detail(&mails, &test_range(), 100, 200).loadouts;
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].loadout.armaments,
            vec![
                LoadoutArmament { id: 2001, value: 10.0 },
                LoadoutArmament { id: 3001, value: 0.026 },
                LoadoutArmament { id: 4001, value: 0.015 },
                LoadoutArmament { id: 5001, value: 0.022 },
            ]
        );
    }
}
