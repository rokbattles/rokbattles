use mongodb::bson::{Bson, Document};

use super::{
    matcher::MatchedArkMailSet,
    types::{
        ArkMatchAlliance, ArkMatchDetail, ArkMatchDetailIndividualResults, ArkMatchDetailOverview,
        ArkMatchDetailPairing, ArkMatchSummary,
    },
};
use crate::{
    bson_utils::{bson_to_f64, bson_to_i64, nested_array, nested_document},
    time_utils::normalize_timestamp_millis,
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct SecondaryWindow {
    pub start_millis: i64,
    pub end_millis: i64,
}

pub(crate) fn extract_mail_time_millis(document: &Document) -> Option<i64> {
    parse_timestamp_millis(nested_value(document, &["metadata", "mail_time"])?)
}

pub(crate) fn extract_mail_times(documents: &[Document]) -> Vec<i64> {
    documents.iter().filter_map(extract_mail_time_millis).collect()
}

pub(crate) fn build_secondary_window(
    mail_times: &[i64],
    max_delta_millis: i64,
) -> Option<SecondaryWindow> {
    if mail_times.is_empty() {
        return None;
    }

    let mut min_time = i64::MAX;
    let mut max_time = i64::MIN;

    for value in mail_times {
        min_time = min_time.min(*value);
        max_time = max_time.max(*value);
    }

    Some(SecondaryWindow {
        start_millis: min_time.saturating_sub(max_delta_millis),
        end_millis: max_time.saturating_add(max_delta_millis).saturating_add(1),
    })
}

pub(crate) fn map_match_record(
    entry: &MatchedArkMailSet,
    fallback_index: usize,
) -> ArkMatchSummary {
    let alliances = nested_array(&entry.battle_results, &["alliances"])
        .into_iter()
        .flatten()
        .filter_map(Bson::as_document)
        .map(map_alliance)
        .collect::<Vec<_>>();

    let self_alliance_id =
        parse_i64(nested_value(&entry.battle_results, &["body", "alliance", "id"]));
    let did_win = parse_bool(nested_value(&entry.battle_results, &["body", "win"]));
    let winner_alliance_id = derive_winner_alliance_id(&alliances, self_alliance_id, did_win);

    let fallback_mail_id =
        format!("{}-{}", entry.battle_results_time_millis, fallback_index.saturating_add(1));

    ArkMatchSummary {
        match_id: entry.battle_results_mail_id.clone().unwrap_or(fallback_mail_id),
        mail_time_millis: entry.battle_results_time_millis,
        battle_results_mail_id: entry.battle_results_mail_id.clone(),
        battle_info_mail_id: entry.battle_info_mail_id.clone(),
        individual_results_mail_id: entry.individual_results_mail_id.clone(),
        alliances,
        winner_alliance_id,
        has_battle_info: entry.battle_info.is_some(),
        has_individual_results: entry.individual_results.is_some(),
    }
}

pub(crate) fn map_match_detail(entry: &MatchedArkMailSet, fallback_index: usize) -> ArkMatchDetail {
    let summary = map_match_record(entry, fallback_index);

    ArkMatchDetail {
        summary,
        overview: map_match_overview(entry.individual_results.as_ref()),
        individual_results: map_individual_results(entry.individual_results.as_ref()),
        pairings: map_pairings(entry.individual_results.as_ref()),
    }
}

fn map_alliance(document: &Document) -> ArkMatchAlliance {
    ArkMatchAlliance {
        id: parse_i64(nested_value(document, &["alliance", "id"])),
        name: parse_string(nested_value(document, &["alliance", "name"])),
        abbreviation: parse_string(nested_value(document, &["alliance", "abbreviation"])),
        score: parse_i64(nested_value(document, &["score"])),
        members: parse_i64(nested_value(document, &["members"])),
        members_max: parse_i64(nested_value(document, &["members_max"])),
        is_blue: parse_bool(nested_value(document, &["is_blue"])),
    }
}

fn derive_winner_alliance_id(
    alliances: &[ArkMatchAlliance],
    self_alliance_id: Option<i64>,
    did_win: Option<bool>,
) -> Option<i64> {
    let self_alliance_id = self_alliance_id?;
    let did_win = did_win?;

    if did_win {
        return Some(self_alliance_id);
    }

    alliances
        .iter()
        .find_map(|alliance| (alliance.id != Some(self_alliance_id)).then_some(alliance.id))
        .flatten()
}

fn map_match_overview(individual_results: Option<&Document>) -> ArkMatchDetailOverview {
    ArkMatchDetailOverview {
        rank: individual_results
            .and_then(|doc| parse_i64(nested_value(doc, &["overview", "rank"]))),
        score: individual_results
            .and_then(|doc| parse_i64(nested_value(doc, &["overview", "score"]))),
        battles: individual_results.and_then(|doc| {
            parse_i64(nested_value(doc, &["overview", "total_results", "battles"]))
        }),
        kill_points_gain: individual_results.and_then(|doc| {
            parse_i64(nested_value(doc, &["overview", "total_results", "kill_points"]))
        }),
        kill_points_loss: individual_results.and_then(|doc| {
            parse_i64(nested_value(doc, &["overview", "total_results", "severely_wounded"]))
        }),
    }
}

fn map_individual_results(
    individual_results: Option<&Document>,
) -> ArkMatchDetailIndividualResults {
    ArkMatchDetailIndividualResults {
        battles_win: individual_results
            .and_then(|doc| parse_i64(nested_value(doc, &["results", "battles_win"]))),
        battles_lose: individual_results
            .and_then(|doc| parse_i64(nested_value(doc, &["results", "battles_lose"]))),
        win_rate: individual_results
            .and_then(|doc| parse_i64(nested_value(doc, &["results", "win_rate"]))),
        kills: individual_results
            .and_then(|doc| parse_i64(nested_value(doc, &["results", "kills"]))),
        severely_wounded: individual_results
            .and_then(|doc| parse_i64(nested_value(doc, &["results", "severely_wounded"]))),
        units_healed: individual_results
            .and_then(|doc| parse_i64(nested_value(doc, &["results", "units_healed"]))),
        speedups: individual_results
            .and_then(|doc| parse_i64(nested_value(doc, &["results", "speedups"]))),
        teleports: individual_results
            .and_then(|doc| parse_i64(nested_value(doc, &["results", "teleports"]))),
        structures: individual_results
            .and_then(|doc| parse_i64(nested_value(doc, &["results", "structures"]))),
        provisions_score: individual_results
            .and_then(|doc| parse_i64(nested_value(doc, &["results", "gather_score"]))),
        ark_of_osiris_score: individual_results
            .and_then(|doc| parse_i64(nested_value(doc, &["results", "flag_score"]))),
        kill_score: individual_results
            .and_then(|doc| parse_i64(nested_value(doc, &["results", "kill_score"]))),
        occupation_score: individual_results
            .and_then(|doc| parse_i64(nested_value(doc, &["results", "building_score"]))),
    }
}

fn map_pairings(individual_results: Option<&Document>) -> Vec<ArkMatchDetailPairing> {
    let Some(individual_results) = individual_results else {
        return Vec::new();
    };

    let Some(pairings) = nested_array(individual_results, &["pairings"]) else {
        return Vec::new();
    };

    pairings
        .iter()
        .filter_map(Bson::as_document)
        .map(|pairing| ArkMatchDetailPairing {
            primary_commander_id: parse_i64(nested_value(pairing, &["primary_commander", "id"])),
            secondary_commander_id: parse_i64(nested_value(
                pairing,
                &["secondary_commander", "id"],
            )),
            battles: parse_i64(nested_value(pairing, &["battles"])),
            battles_win: parse_i64(nested_value(pairing, &["battles_win"])),
            kill_count: parse_i64(nested_value(pairing, &["kill_count"])),
            kill_points: parse_i64(nested_value(pairing, &["kill_points"])),
            severely_wounded: parse_i64(nested_value(pairing, &["severely_wounded"])),
        })
        .collect()
}

fn nested_value<'a>(document: &'a Document, path: &[&str]) -> Option<&'a Bson> {
    if path.is_empty() {
        return None;
    }

    if path.len() == 1 {
        return document.get(path[0]);
    }

    let parent = nested_document(document, &path[..path.len() - 1])?;
    parent.get(path[path.len() - 1])
}

fn parse_timestamp_millis(value: &Bson) -> Option<i64> {
    match value {
        Bson::DateTime(value) => Some(value.timestamp_millis()),
        Bson::String(value) => {
            value.trim().parse::<f64>().ok().and_then(normalize_timestamp_millis)
        }
        other => bson_to_f64(other).and_then(normalize_timestamp_millis),
    }
}

fn parse_i64(value: Option<&Bson>) -> Option<i64> {
    let value = value?;

    match value {
        Bson::String(value) => value
            .trim()
            .parse::<f64>()
            .ok()
            .and_then(|value| if value.is_finite() { Some(value as i64) } else { None }),
        other => bson_to_i64(other),
    }
}

fn parse_bool(value: Option<&Bson>) -> Option<bool> {
    match value? {
        Bson::Boolean(value) => Some(*value),
        Bson::Int32(value) => match value {
            1 => Some(true),
            0 => Some(false),
            _ => None,
        },
        Bson::Int64(value) => match value {
            1 => Some(true),
            0 => Some(false),
            _ => None,
        },
        Bson::Double(value) if value.is_finite() => {
            if *value == 1.0 {
                Some(true)
            } else if *value == 0.0 {
                Some(false)
            } else {
                None
            }
        }
        Bson::String(value) => {
            let normalized = value.trim().to_ascii_lowercase();
            match normalized.as_str() {
                "true" | "1" => Some(true),
                "false" | "0" => Some(false),
                _ => None,
            }
        }
        _ => None,
    }
}

fn parse_string(value: Option<&Bson>) -> Option<String> {
    let value = value?;

    match value {
        Bson::String(value) => {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }
        Bson::Int32(value) => Some(value.to_string()),
        Bson::Int64(value) => Some(value.to_string()),
        Bson::Double(value) if value.is_finite() => Some(value.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use mongodb::bson::doc;

    use super::*;

    #[test]
    fn builds_secondary_window() {
        let window = build_secondary_window(&[1_000, 1_100, 2_000], 50).expect("window");
        assert_eq!(window.start_millis, 950);
        assert_eq!(window.end_millis, 2_051);
    }

    #[test]
    fn maps_match_summary_with_winner() {
        let entry = MatchedArkMailSet {
            battle_results: doc! {
                "metadata": { "mail_id": "m1", "mail_time": 1_000_000_i64 },
                "body": { "win": false, "alliance": { "id": 7_i64 } },
                "alliances": [
                    { "alliance": { "id": 7_i64, "name": "A" }, "is_blue": true },
                    { "alliance": { "id": 8_i64, "name": "B" }, "is_blue": false },
                ]
            },
            battle_results_mail_id: Some("m1".to_string()),
            battle_results_time_millis: 1_000,
            battle_info: None,
            battle_info_mail_id: None,
            individual_results: None,
            individual_results_mail_id: None,
        };

        let mapped = map_match_record(&entry, 0);
        assert_eq!(mapped.winner_alliance_id, Some(8));
        assert_eq!(mapped.match_id, "m1");
    }
}
