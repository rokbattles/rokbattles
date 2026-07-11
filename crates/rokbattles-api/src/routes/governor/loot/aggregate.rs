use std::{collections::HashMap, ops::RangeInclusive};

use mongodb::bson::Bson;

use super::{
    query::{BarbarianLootNpc, BarbarianLootRequest, BaulurLootNpc, FortLootNpc, FortLootRequest},
    store::{
        BarbarianFortMailDocument, BattleMailDocument, BaulurMailDocument,
        KaharTreasureMailDocument, LootEntryDocument,
    },
    types::{LootRewardAggregateResponse, PersonalLootGroupResponse},
};
use crate::{
    bson_utils::bson_to_i64_loose, routes::governor::date_range::GovernorDateRange,
    time_utils::normalize_bson_timestamp_millis,
};

const KAHAR_AP_COST: i64 = 200;

#[derive(Debug, Default)]
struct LootCategoryAggregate {
    reports: i64,
    loot_total: i64,
    ap_used: i64,
    honor_gained: i64,
    xp_gained: i64,
    reward_buckets: HashMap<(i64, i64), LootRewardBucket>,
}

#[derive(Debug)]
struct LootRewardBucket {
    reward_type: i64,
    sub_type: i64,
    total: i64,
    count: i64,
}

pub(crate) fn aggregate_personal_barbarian_loot(
    mails: Vec<BattleMailDocument>,
    request: &BarbarianLootRequest,
) -> Vec<PersonalLootGroupResponse> {
    let group_by_level = request.levels.len() > 1;
    let selected_levels = if request.levels.is_empty() { None } else { Some(&request.levels) };
    let mut groups = HashMap::<Option<i32>, LootCategoryAggregate>::new();

    for mail in mails {
        let Some(event_time_millis) = extract_event_time_millis(
            mail.metadata.as_ref().and_then(|meta| meta.mail_time.as_ref()),
        ) else {
            continue;
        };
        if event_time_millis < request.range.start_millis
            || event_time_millis >= request.range.end_millis
        {
            continue;
        }

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
            if !personal_barbarian_npc_matches(request.npc, npc_type, npc_b_type) {
                continue;
            }
            let Some(metadata) = battle_target_metadata(npc_type, npc_b_type) else {
                continue;
            };
            if selected_levels.is_some_and(|levels| !levels.contains(&metadata.level)) {
                continue;
            }

            let group_key = if group_by_level { Some(metadata.level) } else { None };
            let group = groups.entry(group_key).or_default();
            add_report(group);
            group.ap_used += i64::from(metadata.ap_cost);
            group.honor_gained += i64::from(metadata.honor_points);
            group.xp_gained += npc.experience.as_ref().and_then(parse_i64_loose).unwrap_or(0);
            add_loot(group, npc.loot.as_deref().unwrap_or_default());
        }
    }

    into_personal_groups(groups)
}

pub(crate) fn aggregate_personal_fort_loot(
    mails: Vec<BarbarianFortMailDocument>,
    request: &FortLootRequest,
) -> Vec<PersonalLootGroupResponse> {
    let mut aggregate = LootCategoryAggregate::default();

    for mail in mails {
        let Some(event_time_millis) = extract_event_time_millis(
            mail.metadata.as_ref().and_then(|meta| meta.mail_time.as_ref()),
        ) else {
            continue;
        };
        if event_time_millis < request.range.start_millis
            || event_time_millis >= request.range.end_millis
        {
            continue;
        }
        let Some(metadata) = fort_target_metadata_from_mail(&mail) else {
            continue;
        };
        if !personal_fort_npc_matches(request.npc, metadata.kind) {
            continue;
        }
        if request.level.is_some_and(|level| level != metadata.level) {
            continue;
        }

        add_report(&mut aggregate);
        aggregate.ap_used += i64::from(metadata.ap_cost);
        aggregate.honor_gained += i64::from(metadata.honor_points);
        add_loot(&mut aggregate, mail.rewards.as_deref().unwrap_or_default());
    }

    into_personal_groups(HashMap::from([(None, aggregate)]))
}

pub(crate) fn aggregate_personal_baulur_loot(
    mails: Vec<BaulurMailDocument>,
    governor_id: i64,
    npc: BaulurLootNpc,
    range: &GovernorDateRange,
) -> Vec<PersonalLootGroupResponse> {
    let mut aggregate = LootCategoryAggregate::default();

    for mail in mails {
        let Some(event_time_millis) = extract_event_time_millis(
            mail.metadata.as_ref().and_then(|meta| meta.mail_time.as_ref()),
        ) else {
            continue;
        };
        if event_time_millis < range.start_millis || event_time_millis >= range.end_millis {
            continue;
        }
        let npc_type =
            mail.npc.as_ref().and_then(|npc| npc.npc_type.as_ref()).and_then(parse_i64_loose);
        if !personal_baulur_npc_matches(npc, npc_type) {
            continue;
        }

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
                add_report(&mut aggregate);
                found_matching_participant = true;
            }
            add_loot(&mut aggregate, participant.loot.as_deref().unwrap_or_default());
        }
    }

    into_personal_groups(HashMap::from([(None, aggregate)]))
}

pub(crate) fn aggregate_personal_kahar_treasure_loot(
    mails: Vec<KaharTreasureMailDocument>,
    range: &GovernorDateRange,
) -> Vec<PersonalLootGroupResponse> {
    let mut aggregate = LootCategoryAggregate::default();

    for mail in mails {
        let Some(event_time_millis) = extract_event_time_millis(
            mail.metadata.as_ref().and_then(|meta| meta.mail_time.as_ref()),
        ) else {
            continue;
        };
        if event_time_millis < range.start_millis || event_time_millis >= range.end_millis {
            continue;
        }

        add_report(&mut aggregate);
        aggregate.ap_used += KAHAR_AP_COST;
        add_loot(&mut aggregate, mail.loot.as_deref().unwrap_or_default());
    }

    into_personal_groups(HashMap::from([(None, aggregate)]))
}

fn extract_event_time_millis(mail_time: Option<&Bson>) -> Option<i64> {
    normalize_bson_timestamp_millis(mail_time)
}

fn parse_i64_loose(value: &Bson) -> Option<i64> {
    bson_to_i64_loose(value)
}

const BARBARIAN_GAME_IDS: &[RangeInclusive<i64>] = &[
    1..=40,
    // kvk barbarian
    401..=415,
    // home kingdom barbarian (new variants)
    701..=740,
    801..=840,
    901..=940,
    // english soldier
    150_009..=150_023,
];

fn is_barbarian(npc_type: Option<i64>, npc_b_type: Option<i64>) -> bool {
    if npc_b_type != Some(1) {
        return false;
    }

    let npc_type = match npc_type {
        Some(v) => v,
        None => return false,
    };

    BARBARIAN_GAME_IDS.iter().any(|range| range.contains(&npc_type))
}

fn is_marauder(npc_type: Option<i64>, npc_b_type: Option<i64>) -> bool {
    matches!(npc_type, Some(99 | 100)) && npc_b_type == Some(15)
}

fn add_report(category: &mut LootCategoryAggregate) {
    category.reports += 1;
}

fn add_loot(category: &mut LootCategoryAggregate, loot_entries: &[LootEntryDocument]) {
    if loot_entries.is_empty() {
        return;
    }

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

        let reward_bucket = category
            .reward_buckets
            .entry((reward_type, sub_type))
            .or_insert_with(|| LootRewardBucket { reward_type, sub_type, total: 0, count: 0 });
        reward_bucket.total += value;
        reward_bucket.count += 1;
    }
}

fn into_rewards(category: LootCategoryAggregate) -> Vec<LootRewardAggregateResponse> {
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
    rewards
}

fn into_personal_groups(
    groups: HashMap<Option<i32>, LootCategoryAggregate>,
) -> Vec<PersonalLootGroupResponse> {
    let mut payload = groups
        .into_iter()
        .map(|(level, aggregate)| {
            let reports = aggregate.reports;
            let loot_total = aggregate.loot_total;
            let ap_used = aggregate.ap_used;
            let honor_gained = aggregate.honor_gained;
            let xp_gained = aggregate.xp_gained;
            let rewards = into_rewards(aggregate);
            PersonalLootGroupResponse {
                level,
                reports,
                loot_total,
                ap_used,
                honor_gained,
                xp_gained,
                rewards,
            }
        })
        .collect::<Vec<_>>();
    payload.sort_by_key(|group| group.level);
    payload
}

#[derive(Debug, Clone, Copy)]
struct BattleTargetMetadata {
    level: i32,
    ap_cost: i32,
    honor_points: i32,
}

fn battle_target_metadata(
    npc_type: Option<i64>,
    npc_b_type: Option<i64>,
) -> Option<BattleTargetMetadata> {
    let npc_type = i32::try_from(npc_type?).ok()?;
    let npc_b_type = i32::try_from(npc_b_type?).ok()?;
    let level = match (npc_type, npc_b_type) {
        (1..=40, 1) => npc_type,
        (401..=415, 1) => 41 + npc_type - 401,
        (701..=740, 1) => 1 + npc_type - 701,
        (801..=840, 1) => 1 + npc_type - 801,
        (901..=940, 1) => 1 + npc_type - 901,
        (150_009..=150_023, 1) => 41 + npc_type - 150_009,
        (99, 15) => 1,
        (100, 15) => 41,
        _ => return None,
    };

    Some(BattleTargetMetadata {
        level,
        ap_cost: battle_ap_cost_for_level(level),
        honor_points: battle_honor_points_for_level(level, npc_b_type),
    })
}

fn battle_ap_cost_for_level(level: i32) -> i32 {
    if level >= 41 { 80 } else { 50 }
}

fn battle_honor_points_for_level(level: i32, npc_b_type: i32) -> i32 {
    if npc_b_type == 15 && level == 1 {
        return 0;
    }

    match level {
        41..=45 => 10,
        46..=50 => 16,
        51..=55 => 20,
        _ => 0,
    }
}

fn personal_barbarian_npc_matches(
    npc: BarbarianLootNpc,
    npc_type: Option<i64>,
    npc_b_type: Option<i64>,
) -> bool {
    match npc {
        BarbarianLootNpc::Barbarians => is_barbarian(npc_type, npc_b_type),
        BarbarianLootNpc::Marauders => is_marauder(npc_type, npc_b_type),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FortTargetKind {
    BarbarianFort,
    MarauderEncampment,
    Motte,
}

#[derive(Debug, Clone, Copy)]
struct FortTargetMetadata {
    kind: FortTargetKind,
    level: i32,
    ap_cost: i32,
    honor_points: i32,
}

fn fort_target_metadata_from_mail(mail: &BarbarianFortMailDocument) -> Option<FortTargetMetadata> {
    let sub_param = mail
        .body
        .as_ref()
        .and_then(|body| body.sub_param.as_ref())
        .and_then(parse_i64_loose)
        .and_then(|value| i32::try_from(value).ok())?;
    let level = mail
        .body
        .as_ref()
        .and_then(|body| body.content.as_ref())
        .and_then(|content| content.level.as_ref())
        .and_then(parse_i64_loose)
        .and_then(|value| i32::try_from(value).ok())?;
    let kind = match sub_param {
        1 => FortTargetKind::BarbarianFort,
        3 => FortTargetKind::MarauderEncampment,
        4 => FortTargetKind::Motte,
        _ => return None,
    };
    let ap_cost = if level >= 11 { 300 } else { 150 };
    let honor_points = match kind {
        FortTargetKind::MarauderEncampment if level == 1 => 0,
        _ => fort_honor_points_for_level(level),
    };

    Some(FortTargetMetadata { kind, level, ap_cost, honor_points })
}

fn fort_honor_points_for_level(level: i32) -> i32 {
    match level {
        11 => 30,
        12 => 45,
        13 => 60,
        14 => 80,
        15 => 100,
        _ => 0,
    }
}

fn personal_fort_npc_matches(npc: FortLootNpc, kind: FortTargetKind) -> bool {
    match npc {
        FortLootNpc::BarbarianForts => {
            matches!(kind, FortTargetKind::BarbarianFort | FortTargetKind::Motte)
        }
        FortLootNpc::MarauderEncampments => kind == FortTargetKind::MarauderEncampment,
    }
}

fn personal_baulur_npc_matches(npc: BaulurLootNpc, npc_type: Option<i64>) -> bool {
    match npc {
        BaulurLootNpc::IronhandBaulur => npc_type == Some(102_000_055),
        BaulurLootNpc::MiserKhaolak => npc_type == Some(102_000_063),
    }
}

#[cfg(test)]
mod tests {
    use mongodb::bson::Bson;

    use super::{
        super::store::{
            BarbarianFortBodyDocument, BarbarianFortContentDocument, BarbarianFortMailDocument,
            BattleMailDocument, BattleNpcDocument, BattleOpponentDocument, BaulurMailDocument,
            BaulurNpcDocument, BaulurParticipantDocument, KaharTreasureMailDocument,
            LootEntryDocument, MailMetadataDocument,
        },
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
        add_report(&mut category);

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
        add_loot(&mut category, &loot_entries);

        assert_eq!(category.reports, 1);
        assert_eq!(category.loot_total, 6);
        let rewards = into_rewards(category);
        assert_eq!(rewards.len(), 2);
        assert_eq!(rewards[0].reward_type, 2);
        assert_eq!(rewards[0].sub_type, 26);
        assert_eq!(rewards[0].total, 5);
        assert_eq!(rewards[0].count, 2);
    }

    #[test]
    fn extract_event_time_normalizes_metadata_mail_time() {
        assert_eq!(
            extract_event_time_millis(Some(&Bson::Int64(1_739_960_800))),
            Some(1_739_960_800_000)
        );
    }

    #[test]
    fn aggregate_personal_barbarian_loot_groups_multiple_selected_levels() {
        let range = test_range();
        let battle_time = 1_735_689_600_000;
        let request =
            BarbarianLootRequest { range, npc: BarbarianLootNpc::Barbarians, levels: vec![1, 41] };

        let groups = aggregate_personal_barbarian_loot(
            vec![
                build_npc_mail(battle_time, -2, 1, 1, 5, 10),
                build_npc_mail(battle_time, -2, 401, 1, 7, 11),
                build_npc_mail(battle_time, -2, 2, 1, 13, 12),
            ],
            &request,
        );

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].level, Some(1));
        assert_eq!(groups[0].reports, 1);
        assert_eq!(groups[0].loot_total, 5);
        assert_eq!(groups[0].ap_used, 50);
        assert_eq!(groups[0].xp_gained, 10);
        assert_eq!(groups[1].level, Some(41));
        assert_eq!(groups[1].reports, 1);
        assert_eq!(groups[1].loot_total, 7);
        assert_eq!(groups[1].ap_used, 80);
        assert_eq!(groups[1].honor_gained, 10);
        assert_eq!(groups[1].xp_gained, 11);
    }

    #[test]
    fn battle_honor_points_match_kvktask_rule_visible_values() {
        assert_eq!(battle_honor_points_for_level(41, 1), 10);
        assert_eq!(battle_honor_points_for_level(45, 1), 10);
        assert_eq!(battle_honor_points_for_level(46, 1), 16);
        assert_eq!(battle_honor_points_for_level(50, 1), 16);
        assert_eq!(battle_honor_points_for_level(51, 1), 20);
        assert_eq!(battle_honor_points_for_level(55, 1), 20);
        assert_eq!(battle_honor_points_for_level(41, 15), 10);
    }

    #[test]
    fn aggregate_personal_barbarian_loot_uses_same_honor_for_named_variants() {
        let range = test_range();
        let battle_time = 1_735_689_600_000;
        let request =
            BarbarianLootRequest { range, npc: BarbarianLootNpc::Barbarians, levels: vec![41] };

        let groups = aggregate_personal_barbarian_loot(
            vec![
                build_npc_mail(battle_time, -2, 401, 1, 7, 11),
                build_npc_mail(battle_time, -2, 150_009, 1, 7, 11),
                build_npc_mail(battle_time, -2, 100, 15, 7, 11),
            ],
            &request,
        );

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].reports, 2);
        assert_eq!(groups[0].honor_gained, 20);
    }

    #[test]
    fn aggregate_personal_marauder_loot_uses_level_41_barbarian_honor() {
        let range = test_range();
        let battle_time = 1_735_689_600_000;
        let request =
            BarbarianLootRequest { range, npc: BarbarianLootNpc::Marauders, levels: vec![41] };

        let groups = aggregate_personal_barbarian_loot(
            vec![
                build_npc_mail(battle_time, -2, 401, 1, 7, 11),
                build_npc_mail(battle_time, -2, 100, 15, 7, 11),
            ],
            &request,
        );

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].reports, 1);
        assert_eq!(groups[0].honor_gained, 10);
    }

    #[test]
    fn aggregate_personal_fort_loot_includes_mottes_with_barbarian_forts() {
        let range = test_range();
        let request = FortLootRequest { range, npc: FortLootNpc::BarbarianForts, level: Some(11) };
        let mail_time = 1_735_689_600_000;

        let groups = aggregate_personal_fort_loot(
            vec![
                build_system_barbarian_fort_mail(mail_time, 11, 1, 11),
                build_system_barbarian_fort_mail(mail_time, 17, 4, 11),
                build_system_barbarian_fort_mail(mail_time, 19, 3, 11),
                build_system_barbarian_fort_mail(mail_time, 23, 1, 10),
            ],
            &request,
        );

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].reports, 2);
        assert_eq!(groups[0].loot_total, 28);
        assert_eq!(groups[0].ap_used, 600);
        assert_eq!(groups[0].honor_gained, 60);
    }

    #[test]
    fn aggregate_personal_marauder_encampment_uses_level_11_fort_honor() {
        let range = test_range();
        let request =
            FortLootRequest { range, npc: FortLootNpc::MarauderEncampments, level: Some(11) };
        let mail_time = 1_735_689_600_000;

        let groups = aggregate_personal_fort_loot(
            vec![build_system_barbarian_fort_mail(mail_time, 11, 3, 11)],
            &request,
        );

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].honor_gained, 30);
    }

    #[test]
    fn aggregate_personal_baulur_loot_uses_selected_npc_id() {
        let range = test_range();
        let mail_time = 1_735_689_600_000;

        let groups = aggregate_personal_baulur_loot(
            vec![
                build_baulur_mail(mail_time, 102_000_055, 42, 11),
                build_baulur_mail(mail_time, 102_000_057, 42, 17),
                build_baulur_mail(mail_time, 102_000_063, 42, 19),
            ],
            42,
            BaulurLootNpc::IronhandBaulur,
            &range,
        );

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].reports, 1);
        assert_eq!(groups[0].loot_total, 11);
    }

    #[test]
    fn aggregate_personal_kahar_treasure_loot_counts_mails_without_combat_totals() {
        let range = test_range();
        let mail_time = 1_735_689_600_000;

        let groups = aggregate_personal_kahar_treasure_loot(
            vec![
                build_kahar_treasure_mail(mail_time, 45_000),
                build_kahar_treasure_mail(mail_time, 50_000),
                build_kahar_treasure_mail(1_735_776_000_000, 75_000),
            ],
            &range,
        );

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].reports, 2);
        assert_eq!(groups[0].loot_total, 95_000);
        assert_eq!(groups[0].ap_used, 400);
        assert_eq!(groups[0].honor_gained, 0);
        assert_eq!(groups[0].xp_gained, 0);
    }

    fn build_npc_mail(
        mail_time: i64,
        player_id: i64,
        npc_type: i64,
        npc_b_type: i64,
        loot_value: i64,
        experience: i64,
    ) -> BattleMailDocument {
        BattleMailDocument {
            metadata: Some(MailMetadataDocument { mail_time: Some(Bson::Int64(mail_time)) }),
            opponents: Some(vec![BattleOpponentDocument {
                player_id: Some(Bson::Int64(player_id)),
                npc: Some(BattleNpcDocument {
                    npc_type: Some(Bson::Int64(npc_type)),
                    b_type: Some(Bson::Int64(npc_b_type)),
                    experience: Some(Bson::Int64(experience)),
                    loot: Some(vec![LootEntryDocument {
                        reward_type: Some(Bson::Int64(2)),
                        sub_type: Some(Bson::Int64(26)),
                        value: Some(Bson::Int64(loot_value)),
                    }]),
                }),
            }]),
        }
    }

    fn build_system_barbarian_fort_mail(
        mail_time: i64,
        loot_value: i64,
        sub_param: i64,
        level: i64,
    ) -> BarbarianFortMailDocument {
        BarbarianFortMailDocument {
            metadata: Some(MailMetadataDocument { mail_time: Some(Bson::Int64(mail_time)) }),
            body: Some(BarbarianFortBodyDocument {
                sub_param: Some(Bson::Int64(sub_param)),
                content: Some(BarbarianFortContentDocument { level: Some(Bson::Int64(level)) }),
            }),
            rewards: Some(vec![LootEntryDocument {
                reward_type: Some(Bson::Int64(2)),
                sub_type: Some(Bson::Int64(26)),
                value: Some(Bson::Int64(loot_value)),
            }]),
        }
    }

    fn build_baulur_mail(
        mail_time: i64,
        npc_type: i64,
        player_id: i64,
        loot_value: i64,
    ) -> BaulurMailDocument {
        BaulurMailDocument {
            metadata: Some(MailMetadataDocument { mail_time: Some(Bson::Int64(mail_time)) }),
            npc: Some(BaulurNpcDocument { npc_type: Some(Bson::Int64(npc_type)) }),
            participants: Some(vec![BaulurParticipantDocument {
                player_id: Some(Bson::Int64(player_id)),
                loot: Some(vec![LootEntryDocument {
                    reward_type: Some(Bson::Int64(2)),
                    sub_type: Some(Bson::Int64(26)),
                    value: Some(Bson::Int64(loot_value)),
                }]),
            }]),
        }
    }

    fn build_kahar_treasure_mail(mail_time: i64, loot_value: i64) -> KaharTreasureMailDocument {
        KaharTreasureMailDocument {
            metadata: Some(MailMetadataDocument { mail_time: Some(Bson::Int64(mail_time)) }),
            loot: Some(vec![LootEntryDocument {
                reward_type: Some(Bson::Int64(1)),
                sub_type: Some(Bson::Int64(9)),
                value: Some(Bson::Int64(loot_value)),
            }]),
        }
    }

    fn test_range() -> GovernorDateRange {
        GovernorDateRange {
            start_millis: 1_735_689_600_000,
            end_millis: 1_735_776_000_000,
            start: "2025-01-01".to_string(),
            end: "2025-01-01".to_string(),
        }
    }
}
