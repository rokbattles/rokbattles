use std::collections::{BTreeSet, HashMap};

use core_bson::{nested_array, nested_f64, nested_i64, nested_str};
use mongodb::bson::{Bson, Document};

use crate::{
    routes::governor::{
        date_range::GovernorDateRange,
        pairings::{
            query::{LoadoutGranularity, OpponentGranularity},
            types::{
                EquipmentToken, LoadoutArmament, LoadoutSnapshot, PairingAggregateResponse,
                PairingLoadoutAggregateResponse, PairingOpponentAggregateResponse, PairingTotals,
            },
        },
    },
    time_utils::{normalize_bson_timestamp_millis, normalize_timestamp_millis},
};

#[derive(Debug, Clone)]
struct PairingEntry {
    self_primary_commander_id: i64,
    self_secondary_commander_id: i64,
    enemy_primary_commander_id: i64,
    enemy_secondary_commander_id: i64,
    battle_duration_millis: i64,
    delta: BattleTotalsDelta,
}

#[derive(Debug, Clone, Copy, Default)]
struct BattleTotalsDelta {
    kill_score: i64,
    deaths: i64,
    severely_wounded: i64,
    wounded: i64,
    healing_count: i64,
    enemy_kill_score: i64,
    enemy_deaths: i64,
    enemy_severely_wounded: i64,
    enemy_wounded: i64,
}

pub(crate) fn aggregate_pairings(
    mails: &[Document],
    range: &GovernorDateRange,
) -> Vec<PairingAggregateResponse> {
    let mut buckets: HashMap<(i64, i64), PairingAggregateResponse> = HashMap::new();

    for mail in mails {
        if !mail_is_in_range(mail, range) {
            continue;
        }

        for_each_pairing_entry(mail, |entry| {
            if entry.self_primary_commander_id <= 0 {
                return;
            }

            let key = (entry.self_primary_commander_id, entry.self_secondary_commander_id);
            let bucket = buckets.entry(key).or_insert_with(|| PairingAggregateResponse {
                primary_commander_id: entry.self_primary_commander_id,
                secondary_commander_id: entry.self_secondary_commander_id,
                count: 0,
                totals: PairingTotals::default(),
            });

            bucket.count += 1;
            apply_battle_delta(&mut bucket.totals, entry.delta, entry.battle_duration_millis);
        });
    }

    let mut items = buckets.into_values().collect::<Vec<_>>();
    finalize_totals(&mut items, |item| (&mut item.totals, item.count));
    sort_by_kill_score_then_count(&mut items, |item| (&item.totals, item.count));
    items
}

pub(crate) fn aggregate_loadouts(
    mails: &[Document],
    range: &GovernorDateRange,
    primary_commander_id: i64,
    secondary_commander_id: i64,
    granularity: LoadoutGranularity,
) -> Vec<PairingLoadoutAggregateResponse> {
    let mut buckets: HashMap<String, PairingLoadoutAggregateResponse> = HashMap::new();

    for mail in mails {
        if !mail_is_in_range(mail, range) {
            continue;
        }

        let loadout = build_loadout_snapshot(mail, granularity);
        let key = build_loadout_key(&loadout);

        for_each_pairing_entry(mail, |entry| {
            if entry.self_primary_commander_id != primary_commander_id
                || entry.self_secondary_commander_id != secondary_commander_id
            {
                return;
            }

            let bucket =
                buckets.entry(key.clone()).or_insert_with(|| PairingLoadoutAggregateResponse {
                    key: key.clone(),
                    count: 0,
                    totals: PairingTotals::default(),
                    loadout: loadout.clone(),
                });

            bucket.count += 1;
            apply_battle_delta(&mut bucket.totals, entry.delta, entry.battle_duration_millis);
        });
    }

    let mut items = buckets.into_values().collect::<Vec<_>>();
    finalize_totals(&mut items, |item| (&mut item.totals, item.count));
    sort_by_kill_score_then_count(&mut items, |item| (&item.totals, item.count));
    items
}

pub(crate) fn aggregate_opponents(
    mails: &[Document],
    range: &GovernorDateRange,
    primary_commander_id: i64,
    secondary_commander_id: i64,
    granularity: OpponentGranularity,
    loadout_key: Option<&str>,
) -> Vec<PairingOpponentAggregateResponse> {
    let mut buckets: HashMap<(i64, i64), PairingOpponentAggregateResponse> = HashMap::new();

    for mail in mails {
        if !mail_is_in_range(mail, range) {
            continue;
        }

        if granularity != OpponentGranularity::Overall {
            let lookup_granularity = match granularity {
                OpponentGranularity::Simplified => LoadoutGranularity::Simplified,
                OpponentGranularity::Exact => LoadoutGranularity::Exact,
                OpponentGranularity::Overall => LoadoutGranularity::Exact,
            };
            let snapshot = build_loadout_snapshot(mail, lookup_granularity);
            let key = build_loadout_key(&snapshot);
            if Some(key.as_str()) != loadout_key {
                continue;
            }
        }

        let mut found_matching_entry = false;
        for_each_pairing_entry(mail, |entry| {
            if entry.self_primary_commander_id != primary_commander_id
                || entry.self_secondary_commander_id != secondary_commander_id
            {
                return;
            }

            found_matching_entry = true;
            let key = (entry.enemy_primary_commander_id, entry.enemy_secondary_commander_id);
            let bucket = buckets.entry(key).or_insert_with(|| PairingOpponentAggregateResponse {
                enemy_primary_commander_id: entry.enemy_primary_commander_id,
                enemy_secondary_commander_id: entry.enemy_secondary_commander_id,
                count: 0,
                totals: PairingTotals::default(),
            });
            bucket.count += 1;
            apply_battle_delta(&mut bucket.totals, entry.delta, entry.battle_duration_millis);
        });

        if !found_matching_entry {
            continue;
        }
    }

    let mut items = buckets.into_values().collect::<Vec<_>>();
    finalize_totals(&mut items, |item| (&mut item.totals, item.count));
    sort_by_kill_score_then_count(&mut items, |item| (&item.totals, item.count));
    items
}

fn mail_is_in_range(mail: &Document, range: &GovernorDateRange) -> bool {
    extract_event_time_millis(mail)
        .map(|event_time| event_time >= range.start_millis && event_time < range.end_millis)
        .unwrap_or(false)
}

fn extract_event_time_millis(mail: &Document) -> Option<i64> {
    normalize_bson_timestamp_millis(mail.get_document("metadata").ok()?.get("mail_time"))
}

fn for_each_pairing_entry(mail: &Document, mut push: impl FnMut(PairingEntry)) {
    let self_primary_commander_id =
        nested_i64(mail, &["sender", "commanders", "primary", "id"]).unwrap_or_default();
    let self_secondary_commander_id =
        nested_i64(mail, &["sender", "commanders", "secondary", "id"]).unwrap_or_default();

    let mut opponents = nested_array(mail, &["opponents"])
        .into_iter()
        .flatten()
        .filter_map(Bson::as_document)
        .filter(|opponent| {
            let player_id = nested_i64(opponent, &["player_id"]).unwrap_or_default();
            player_id > 0
        })
        .collect::<Vec<_>>();

    opponents.sort_by(|left, right| {
        let left_start = nested_i64(left, &["start_tick"]).unwrap_or_default();
        let right_start = nested_i64(right, &["start_tick"]).unwrap_or_default();
        if left_start != right_start {
            return left_start.cmp(&right_start);
        }

        let left_player = nested_i64(left, &["player_id"]).unwrap_or_default();
        let right_player = nested_i64(right, &["player_id"]).unwrap_or_default();
        left_player.cmp(&right_player)
    });

    for opponent in opponents {
        push(PairingEntry {
            self_primary_commander_id,
            self_secondary_commander_id,
            enemy_primary_commander_id: nested_i64(opponent, &["commanders", "primary", "id"])
                .unwrap_or_default(),
            enemy_secondary_commander_id: nested_i64(opponent, &["commanders", "secondary", "id"])
                .unwrap_or_default(),
            battle_duration_millis: extract_battle_duration_millis(mail, opponent),
            delta: extract_battle_delta(opponent),
        });
    }
}

fn extract_battle_duration_millis(mail: &Document, opponent: &Document) -> i64 {
    let timeline_start = nested_f64(mail, &["timeline", "start_timestamp"]).unwrap_or_default();
    let start_tick = nested_f64(opponent, &["start_tick"]).unwrap_or_default();
    let end_tick = nested_f64(opponent, &["end_tick"]).unwrap_or(start_tick);

    let start = normalize_timestamp_millis(timeline_start + start_tick);
    let end = normalize_timestamp_millis(timeline_start + end_tick);
    match (start, end) {
        (Some(start), Some(end)) if end > start => end - start,
        _ => 0,
    }
}

fn extract_battle_delta(opponent: &Document) -> BattleTotalsDelta {
    BattleTotalsDelta {
        kill_score: nested_i64(opponent, &["battle_results", "sender", "kill_points"])
            .unwrap_or_default(),
        deaths: nested_i64(opponent, &["battle_results", "sender", "dead"]).unwrap_or_default(),
        severely_wounded: nested_i64(opponent, &["battle_results", "sender", "severely_wounded"])
            .unwrap_or_default(),
        wounded: nested_i64(opponent, &["battle_results", "sender", "slightly_wounded"])
            .unwrap_or_default(),
        healing_count: nested_i64(opponent, &["battle_results", "sender", "heal"])
            .unwrap_or_default(),
        enemy_kill_score: nested_i64(opponent, &["battle_results", "opponent", "kill_points"])
            .unwrap_or_default(),
        enemy_deaths: nested_i64(opponent, &["battle_results", "opponent", "dead"])
            .unwrap_or_default(),
        enemy_severely_wounded: nested_i64(
            opponent,
            &["battle_results", "opponent", "severely_wounded"],
        )
        .unwrap_or_default(),
        enemy_wounded: nested_i64(opponent, &["battle_results", "opponent", "slightly_wounded"])
            .unwrap_or_default(),
    }
}

fn apply_battle_delta(totals: &mut PairingTotals, delta: BattleTotalsDelta, battle_duration: i64) {
    totals.kill_score += delta.kill_score;
    totals.deaths += delta.deaths;
    totals.severely_wounded += delta.severely_wounded;
    totals.wounded += delta.wounded;
    totals.healing_count += delta.healing_count;
    totals.enemy_kill_score += delta.enemy_kill_score;
    totals.enemy_deaths += delta.enemy_deaths;
    totals.enemy_severely_wounded += delta.enemy_severely_wounded;
    totals.enemy_wounded += delta.enemy_wounded;
    totals.dps += delta.enemy_wounded + delta.enemy_severely_wounded;
    totals.sps += delta.enemy_severely_wounded;
    totals.tps += delta.severely_wounded;
    totals.battle_duration += battle_duration;
    totals.trade_percent_total += if delta.kill_score == delta.enemy_kill_score {
        100
    } else if delta.enemy_kill_score <= 0 {
        0
    } else {
        ((delta.kill_score as f64 / delta.enemy_kill_score as f64) * 100.0).round() as i64
    };
}

fn finalize_totals<T>(items: &mut [T], lookup: impl Fn(&mut T) -> (&mut PairingTotals, i64)) {
    for item in items {
        let (totals, count) = lookup(item);
        totals.trade_percent =
            if count > 0 { totals.trade_percent_total as f64 / count as f64 } else { 0.0 };
        totals.weighted_trade_percent =
            compute_trade_percent(totals.kill_score, totals.enemy_kill_score);
        totals.hps = if totals.battle_duration > 0 {
            totals.healing_count as f64 / (totals.battle_duration as f64 / 1000.0)
        } else {
            0.0
        };
    }
}

fn compute_trade_percent(kill_score: i64, enemy_kill_score: i64) -> f64 {
    if kill_score == enemy_kill_score {
        100.0
    } else if enemy_kill_score <= 0 {
        0.0
    } else {
        (kill_score as f64 / enemy_kill_score as f64) * 100.0
    }
}

fn build_loadout_snapshot(mail: &Document, granularity: LoadoutGranularity) -> LoadoutSnapshot {
    let equipment =
        parse_equipment(nested_str(mail, &["sender", "commanders", "primary", "equipment"]));
    let formation = nested_i64(mail, &["sender", "commanders", "primary", "formation"])
        .and_then(|value| (value > 0).then_some(value));

    let mut inscription_ids = BTreeSet::new();
    let mut buff_totals: HashMap<i64, f64> = HashMap::new();

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

    match granularity {
        LoadoutGranularity::Simplified => LoadoutSnapshot {
            equipment: normalize_equipment_tokens(&equipment),
            armaments: armament_pairs
                .into_iter()
                .map(|(id, _)| LoadoutArmament { id, value: None })
                .collect::<Vec<_>>(),
            inscriptions,
            formation,
        },
        LoadoutGranularity::Exact => LoadoutSnapshot {
            equipment,
            armaments: armament_pairs
                .into_iter()
                .map(|(id, value)| LoadoutArmament { id, value: Some(value) })
                .collect::<Vec<_>>(),
            inscriptions,
            formation,
        },
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

fn normalize_equipment_tokens(tokens: &[EquipmentToken]) -> Vec<EquipmentToken> {
    tokens
        .iter()
        .map(|token| EquipmentToken {
            slot: token.slot,
            id: token.id,
            craft: token.craft,
            attr: token.attr.map(normalize_equipment_attr),
        })
        .collect::<Vec<_>>()
}

fn normalize_equipment_attr(attr: i64) -> i64 {
    let base = attr / 10;
    if base > 0 { base * 10 } else { 0 }
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
            let parts =
                token.split(|ch| ['_', ':'].contains(&ch)).map(str::trim).collect::<Vec<_>>();
            if parts.len() < 2 {
                return None;
            }

            let buff_id = parts[0].parse::<i64>().ok()?;
            let buff_value = parts[1].parse::<f64>().ok()?;
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
        .map(|buff| match buff.value {
            Some(value) => format!("{}_{}", buff.id, format_armament_value(value)),
            None => buff.id.to_string(),
        })
        .collect::<Vec<_>>()
        .join("|")
}

fn format_armament_value(value: f64) -> String {
    let rounded = (value * 1_000_000.0).round() / 1_000_000.0;
    format!("{rounded:.6}").trim_end_matches('0').trim_end_matches('.').to_string()
}

fn sort_by_kill_score_then_count<T>(items: &mut [T], lookup: impl Fn(&T) -> (&PairingTotals, i64)) {
    items.sort_by(|left, right| {
        let (left_totals, left_count) = lookup(left);
        let (right_totals, right_count) = lookup(right);
        right_totals
            .kill_score
            .cmp(&left_totals.kill_score)
            .then_with(|| right_count.cmp(&left_count))
    });
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
                            "heal": 25_i64
                        },
                        "opponent": {
                            "kill_points": enemy_kill_score,
                            "dead": 3_i64,
                            "severely_wounded": 11_i64,
                            "slightly_wounded": 13_i64
                        }
                    }
                }
            ]
        }
    }

    #[test]
    fn aggregate_pairings_groups_by_pairing_and_accumulates_totals() {
        let mails = vec![
            build_test_mail("{1:100_2:25}", "1000_2;1001_3"),
            build_test_mail_with_scores("{1:100_2:25}", "1000_2;1001_3", 100, 200),
        ];

        let items = aggregate_pairings(&mails, &test_range());
        assert_eq!(items.len(), 1);
        let first = &items[0];
        assert_eq!(first.primary_commander_id, 100);
        assert_eq!(first.secondary_commander_id, 200);
        assert_eq!(first.count, 2);
        assert_eq!(first.totals.kill_score, 200);
        assert_eq!(first.totals.healing_count, 50);
        assert_eq!(first.totals.enemy_severely_wounded, 22);
        assert_eq!(first.totals.dps, 48);
        assert_eq!(first.totals.trade_percent, 125.0);
        assert_eq!(first.totals.weighted_trade_percent, 80.0);
        assert_eq!(first.totals.hps, 5.0);
    }

    #[test]
    fn aggregate_loadouts_treats_simplified_and_exact_differently() {
        let mail_one = build_test_mail("{1:100_2:21}", "1000_2;1001_3");
        let mail_two = build_test_mail("{1:100_2:29}", "1000_9;1001_3");
        let mails = vec![mail_one, mail_two];

        let simplified =
            aggregate_loadouts(&mails, &test_range(), 100, 200, LoadoutGranularity::Simplified);
        let exact = aggregate_loadouts(&mails, &test_range(), 100, 200, LoadoutGranularity::Exact);

        assert_eq!(simplified.len(), 1);
        assert_eq!(exact.len(), 2);
    }

    #[test]
    fn aggregate_loadouts_parses_decimal_armament_buff_values() {
        let mails =
            vec![build_test_mail("{1:100_2:21}", "3001_0.026000;4001_0.015000;5001_0.022000")];

        let simplified =
            aggregate_loadouts(&mails, &test_range(), 100, 200, LoadoutGranularity::Simplified);
        let exact = aggregate_loadouts(&mails, &test_range(), 100, 200, LoadoutGranularity::Exact);

        assert_eq!(simplified.len(), 1);
        assert_eq!(exact.len(), 1);

        assert!(!simplified[0].loadout.armaments.is_empty());
        assert!(simplified[0].loadout.armaments.iter().all(|armament| armament.value.is_none()));
        assert!(exact[0].loadout.armaments.iter().any(|armament| armament.value.is_some()));
    }

    #[test]
    fn aggregate_opponents_filters_by_loadout_key() {
        let target_mail = build_test_mail("{1:100_2:25}", "1000_2;1001_3");
        let other_mail = build_test_mail("{1:100_2:30}", "1000_2;1001_3");
        let loadout_key =
            build_loadout_key(&build_loadout_snapshot(&target_mail, LoadoutGranularity::Exact));

        let items = aggregate_opponents(
            &[target_mail, other_mail],
            &test_range(),
            100,
            200,
            OpponentGranularity::Exact,
            Some(&loadout_key),
        );

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].count, 1);
    }
}
