use std::collections::HashMap;

use mongodb::bson::Bson;

use super::{
    store::{BarbarianFortMailDocument, BattleMailDocument, BaulurMailDocument, LootEntryDocument},
    types::{
        LootCategories, LootCategoryAggregateResponse, LootDailyAggregateResponse,
        LootRewardAggregateResponse,
    },
};
use crate::{
    bson_utils::bson_to_i64_loose,
    routes::governor::date_range::GovernorDateRange,
    time_utils::{date_key_utc, normalize_bson_timestamp_millis},
};

#[derive(Debug, Default)]
struct LootCategoryAggregate {
    reports: i64,
    loot_total: i64,
    reward_buckets: HashMap<(i64, i64), LootRewardBucket>,
    daily_buckets: HashMap<String, LootDailyBucket>,
}

#[derive(Debug)]
struct LootRewardBucket {
    reward_type: i64,
    sub_type: i64,
    total: i64,
    count: i64,
}

#[derive(Debug)]
struct LootDailyBucket {
    date: String,
    reports: i64,
    loot_total: i64,
}

pub(crate) fn aggregate_loot(
    barbarian_mails: Vec<BattleMailDocument>,
    marauder_mails: Vec<BattleMailDocument>,
    barbarian_fort_mails: Vec<BarbarianFortMailDocument>,
    baulur_mails: Vec<BaulurMailDocument>,
    governor_id: i64,
    range: &GovernorDateRange,
) -> LootCategories {
    let mut barbarian = LootCategoryAggregate::default();
    let mut marauder = LootCategoryAggregate::default();
    let mut barbarian_fort = LootCategoryAggregate::default();
    let mut baulur = LootCategoryAggregate::default();

    aggregate_npc_battle_loot(barbarian_mails, range, &mut barbarian, is_barbarian);
    aggregate_npc_battle_loot(marauder_mails, range, &mut marauder, is_marauder);

    for mail in barbarian_fort_mails {
        let Some(event_time_millis) = extract_event_time_millis(
            mail.metadata.as_ref().and_then(|meta| meta.mail_time.as_ref()),
        ) else {
            continue;
        };
        if event_time_millis < range.start_millis || event_time_millis >= range.end_millis {
            continue;
        }
        let Some(date_key) = date_key_utc(event_time_millis) else {
            continue;
        };

        add_report(&mut barbarian_fort, &date_key);
        add_loot(&mut barbarian_fort, &date_key, mail.rewards.as_deref().unwrap_or_default());
    }

    for mail in baulur_mails {
        let Some(event_time_millis) = extract_event_time_millis(
            mail.metadata.as_ref().and_then(|meta| meta.mail_time.as_ref()),
        ) else {
            continue;
        };
        if event_time_millis < range.start_millis || event_time_millis >= range.end_millis {
            continue;
        }
        let Some(date_key) = date_key_utc(event_time_millis) else {
            continue;
        };

        let mut found_matching_participant = false;
        for participant in mail.participants.unwrap_or_default() {
            let Some(participant_id) = participant.player_id.as_ref().and_then(parse_i64_loose)
            else {
                continue;
            };
            if participant_id != governor_id {
                continue;
            }

            if !found_matching_participant {
                add_report(&mut baulur, &date_key);
                found_matching_participant = true;
            }
            add_loot(&mut baulur, &date_key, participant.loot.as_deref().unwrap_or_default());
        }
    }

    LootCategories {
        barbarian: into_category_payload(barbarian),
        marauder: into_category_payload(marauder),
        barbarian_fort: into_category_payload(barbarian_fort),
        baulur: into_category_payload(baulur),
    }
}

fn aggregate_npc_battle_loot(
    mails: Vec<BattleMailDocument>,
    range: &GovernorDateRange,
    category: &mut LootCategoryAggregate,
    npc_matches_category: fn(Option<i64>, Option<i64>) -> bool,
) {
    for mail in mails {
        let Some(event_time_millis) = extract_event_time_millis(
            mail.metadata.as_ref().and_then(|meta| meta.mail_time.as_ref()),
        ) else {
            continue;
        };
        if event_time_millis < range.start_millis || event_time_millis >= range.end_millis {
            continue;
        }
        let Some(date_key) = date_key_utc(event_time_millis) else {
            continue;
        };

        for opponent in mail.opponents.unwrap_or_default() {
            let Some(opponent_id) = opponent.player_id.as_ref().and_then(parse_i64_loose) else {
                continue;
            };
            if opponent_id != -2 {
                continue;
            }
            let Some(npc) = opponent.npc else {
                continue;
            };

            let npc_type = npc.npc_type.as_ref().and_then(parse_i64_loose);
            let npc_b_type = npc.b_type.as_ref().and_then(parse_i64_loose);
            if !npc_matches_category(npc_type, npc_b_type) {
                continue;
            }

            add_report(category, &date_key);
            add_loot(category, &date_key, npc.loot.as_deref().unwrap_or_default());
        }
    }
}

fn extract_event_time_millis(mail_time: Option<&Bson>) -> Option<i64> {
    normalize_bson_timestamp_millis(mail_time)
}

fn parse_i64_loose(value: &Bson) -> Option<i64> {
    bson_to_i64_loose(value)
}

fn is_barbarian(npc_type: Option<i64>, npc_b_type: Option<i64>) -> bool {
    if npc_type.is_none() || npc_b_type != Some(1) {
        return false;
    }
    let npc_type = npc_type.unwrap_or_default();
    let is_home_barbarian = (1..=40).contains(&npc_type);
    let is_kvk_barbarian = (401..=415).contains(&npc_type);
    let is_english_soldier_barbarian = (150_009..=150_023).contains(&npc_type);
    is_home_barbarian || is_kvk_barbarian || is_english_soldier_barbarian
}

fn is_marauder(npc_type: Option<i64>, npc_b_type: Option<i64>) -> bool {
    matches!(npc_type, Some(99 | 100)) && npc_b_type == Some(15)
}

fn add_report(category: &mut LootCategoryAggregate, date_key: &str) {
    category.reports += 1;
    let daily_bucket = category.daily_buckets.entry(date_key.to_string()).or_insert_with(|| {
        LootDailyBucket { date: date_key.to_string(), reports: 0, loot_total: 0 }
    });
    daily_bucket.reports += 1;
}

fn add_loot(
    category: &mut LootCategoryAggregate,
    date_key: &str,
    loot_entries: &[LootEntryDocument],
) {
    if loot_entries.is_empty() {
        return;
    }

    let daily_bucket = category.daily_buckets.entry(date_key.to_string()).or_insert_with(|| {
        LootDailyBucket { date: date_key.to_string(), reports: 0, loot_total: 0 }
    });

    for entry in loot_entries {
        let Some(reward_type) = entry.reward_type.as_ref().and_then(parse_i64_loose) else {
            continue;
        };
        let Some(sub_type) = entry.sub_type.as_ref().and_then(parse_i64_loose) else {
            continue;
        };
        let Some(value) = entry.value.as_ref().and_then(parse_i64_loose) else {
            continue;
        };

        category.loot_total += value;
        daily_bucket.loot_total += value;

        let reward_bucket = category
            .reward_buckets
            .entry((reward_type, sub_type))
            .or_insert_with(|| LootRewardBucket { reward_type, sub_type, total: 0, count: 0 });
        reward_bucket.total += value;
        reward_bucket.count += 1;
    }
}

fn into_category_payload(category: LootCategoryAggregate) -> LootCategoryAggregateResponse {
    let mut rewards = category
        .reward_buckets
        .into_values()
        .map(|reward| LootRewardAggregateResponse {
            reward_type: reward.reward_type,
            sub_type: reward.sub_type,
            total: reward.total,
            count: reward.count,
        })
        .collect::<Vec<_>>();
    rewards.sort_by(|left, right| {
        left.reward_type.cmp(&right.reward_type).then(left.sub_type.cmp(&right.sub_type))
    });

    let mut daily = category
        .daily_buckets
        .into_values()
        .map(|value| LootDailyAggregateResponse {
            date: value.date,
            reports: value.reports,
            loot_total: value.loot_total,
        })
        .collect::<Vec<_>>();
    daily.sort_by(|left, right| left.date.cmp(&right.date));

    LootCategoryAggregateResponse {
        reports: category.reports,
        loot_total: category.loot_total,
        rewards,
        daily,
    }
}

#[cfg(test)]
mod tests {
    use mongodb::bson::Bson;

    use super::{
        super::store::{BattleNpcDocument, BattleOpponentDocument, MailMetadataDocument},
        *,
    };

    #[test]
    fn normalize_timestamp_supports_seconds_millis_and_micros() {
        assert_eq!(
            extract_event_time_millis(Some(&Bson::Int64(1_739_960_800))),
            Some(1_739_960_800_000)
        );
        assert_eq!(
            extract_event_time_millis(Some(&Bson::Int64(1_739_960_800_000))),
            Some(1_739_960_800_000)
        );
        assert_eq!(
            extract_event_time_millis(Some(&Bson::Int64(1_739_960_800_000_000))),
            Some(1_739_960_800_000)
        );
    }

    #[test]
    fn is_barbarian_matches_supported_npc_ranges() {
        assert!(is_barbarian(Some(1), Some(1)));
        assert!(is_barbarian(Some(415), Some(1)));
        assert!(is_barbarian(Some(150_020), Some(1)));
        assert!(!is_barbarian(Some(500), Some(1)));
        assert!(!is_barbarian(Some(1), Some(2)));
    }

    #[test]
    fn is_marauder_matches_expected_npc_type_and_b_type() {
        assert!(is_marauder(Some(99), Some(15)));
        assert!(is_marauder(Some(100), Some(15)));
        assert!(!is_marauder(Some(99), Some(1)));
        assert!(!is_marauder(Some(1), Some(15)));
    }

    #[test]
    fn add_loot_aggregates_totals_and_drops() {
        let mut category = LootCategoryAggregate::default();
        add_report(&mut category, "2025-01-01");

        let loot_entries = vec![
            LootEntryDocument {
                reward_type: Some(Bson::Int32(2)),
                sub_type: Some(Bson::Int32(26)),
                value: Some(Bson::Int64(3)),
            },
            LootEntryDocument {
                reward_type: Some(Bson::Int32(2)),
                sub_type: Some(Bson::Int64(26)),
                value: Some(Bson::Int64(2)),
            },
            LootEntryDocument {
                reward_type: Some(Bson::Int32(2)),
                sub_type: Some(Bson::Int64(44)),
                value: Some(Bson::Int64(1)),
            },
        ];
        add_loot(&mut category, "2025-01-01", &loot_entries);

        let payload = into_category_payload(category);
        assert_eq!(payload.reports, 1);
        assert_eq!(payload.loot_total, 6);
        assert_eq!(payload.daily.len(), 1);
        assert_eq!(payload.daily[0].loot_total, 6);
        assert_eq!(payload.rewards.len(), 2);
        assert_eq!(payload.rewards[0].reward_type, 2);
        assert_eq!(payload.rewards[0].sub_type, 26);
        assert_eq!(payload.rewards[0].total, 5);
        assert_eq!(payload.rewards[0].count, 2);
    }

    #[test]
    fn extract_event_time_normalizes_metadata_mail_time() {
        assert_eq!(
            extract_event_time_millis(Some(&Bson::Int64(1_739_960_800))),
            Some(1_739_960_800_000)
        );
    }

    #[test]
    fn aggregate_loot_tracks_marauders_separately_from_barbarians() {
        let range = GovernorDateRange {
            start_millis: 1_735_689_600_000,
            end_millis: 1_735_776_000_000,
            start: "2025-01-01".to_string(),
            end: "2025-01-01".to_string(),
        };
        let battle_time = 1_735_689_600_000;

        let categories = aggregate_loot(
            vec![build_npc_mail(battle_time, -2, 1, 1, 5)],
            vec![
                build_npc_mail(battle_time, -2, 99, 15, 7),
                build_npc_mail(battle_time, -2, 100, 15, 13),
                build_npc_mail(battle_time, -1, 99, 15, 11),
            ],
            Vec::new(),
            Vec::new(),
            42,
            &range,
        );

        assert_eq!(categories.barbarian.reports, 1);
        assert_eq!(categories.barbarian.loot_total, 5);
        assert_eq!(categories.marauder.reports, 2);
        assert_eq!(categories.marauder.loot_total, 20);
        assert_eq!(categories.total_reports(), 3);
    }

    fn build_npc_mail(
        mail_time: i64,
        player_id: i64,
        npc_type: i64,
        npc_b_type: i64,
        loot_value: i64,
    ) -> BattleMailDocument {
        BattleMailDocument {
            metadata: Some(MailMetadataDocument { mail_time: Some(Bson::Int64(mail_time)) }),
            opponents: Some(vec![BattleOpponentDocument {
                player_id: Some(Bson::Int64(player_id)),
                npc: Some(BattleNpcDocument {
                    npc_type: Some(Bson::Int64(npc_type)),
                    b_type: Some(Bson::Int64(npc_b_type)),
                    loot: Some(vec![LootEntryDocument {
                        reward_type: Some(Bson::Int64(2)),
                        sub_type: Some(Bson::Int64(26)),
                        value: Some(Bson::Int64(loot_value)),
                    }]),
                }),
            }]),
        }
    }
}
