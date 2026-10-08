use mongodb::bson::{Bson, Document};
use rokbattles_bson::{bson_to_f64, bson_to_i64, nested_array, nested_document, nested_value};

use super::{
    matcher::MatchedArkMailSet,
    types::{
        ArkHighlight, ArkLeague, ArkMatchAlliance, ArkMatchDetail, ArkMatchDetailIndividualResults,
        ArkMatchDetailOverview, ArkMatchDetailPairing, ArkMatchSummary, ArkParticipant,
        ArkReportCoverage, ArkReportStatus,
    },
};
use crate::time_utils::normalize_timestamp_millis;

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
        alliances,
        winner_alliance_id,
        self_alliance_id,
        league: league_from_mail(&entry.battle_results),
        personal_score: map_match_overview(entry.individual_results.as_ref()).score,
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
        participants: map_participants(&entry.battle_results),
        highlights: map_highlights(&entry.battle_results),
        battle_reports: ArkReportCoverage { status: ArkReportStatus::Missing, total: 0 },
    }
}

fn map_alliance(document: &Document) -> ArkMatchAlliance {
    ArkMatchAlliance {
        id: parse_i64(nested_value(document, &["alliance", "id"])),
        name: parse_string(nested_value(document, &["alliance", "name"])),
        logo: parse_string(nested_value(document, &["alliance", "logo"])),
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
            .and_then(|doc| parse_i64(nested_value(doc, &["overview", "rank"])))
            .filter(|rank| *rank > 0),
        score: individual_results
            .and_then(|doc| {
                parse_i64(nested_value(doc, &["overview", "score"]))
                    .or_else(|| parse_i64(nested_value(doc, &["results", "total_score"])))
            })
            .filter(|score| *score >= 0),
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
        speedups_minutes: individual_results
            .and_then(|doc| parse_i64(nested_value(doc, &["results", "speedups"]))),
        teleports: individual_results
            .and_then(|doc| parse_i64(nested_value(doc, &["results", "teleports"]))),
        provisions_score: individual_results
            .and_then(|doc| parse_i64(nested_value(doc, &["results", "gather_score"]))),
        ark_of_osiris_score: individual_results
            .and_then(|doc| parse_i64(nested_value(doc, &["results", "flag_score"]))),
        kill_score: individual_results
            .and_then(|doc| parse_i64(nested_value(doc, &["results", "kill_score"]))),
        healing_score: individual_results
            .and_then(|doc| parse_i64(nested_value(doc, &["results", "healing_score"]))),
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
            loss_points: parse_i64(nested_value(pairing, &["severely_wounded"])),
        })
        .collect()
}

fn league_from_mail(document: &Document) -> ArkLeague {
    match nested_value(document, &["body", "battle_type"]).and_then(Bson::as_str) {
        Some("EgyptLeague") => ArkLeague::Osiris,
        Some("diy_egypt" | "DiyEgypt" | "InviteMatch") => ArkLeague::Custom,
        Some("Practice" | "InvitePractice") => ArkLeague::Practice,
        Some("SilverEgypt") => ArkLeague::Silver,
        // Egypt/Egype does not distinguish Golden from Silver in older stored mail.
        _ => ArkLeague::Unknown,
    }
}

fn nonnegative_number(value: Option<&Bson>) -> Option<f64> {
    let number = rokbattles_bson::bson_to_f64_loose(value?)?;
    (number.is_finite() && number >= 0.0).then_some(number)
}

fn map_participants(document: &Document) -> Vec<ArkParticipant> {
    nested_array(document, &["participants"])
        .into_iter()
        .flatten()
        .filter_map(Bson::as_document)
        .enumerate()
        .map(|(index, participant)| ArkParticipant {
            rank: index + 1,
            name: parse_string(participant.get("player_name")),
            participated: participant
                .get("individual_points")
                .and_then(rokbattles_bson::bson_to_f64_loose)
                .filter(|value| value.is_finite())
                .map(|value| value >= 0.0),
            // Negative scores mean the governor did not enter the match.
            score: nonnegative_number(participant.get("individual_points")),
            occupation_score: nonnegative_number(participant.get("building_score")),
            provisions_score: nonnegative_number(participant.get("gather_score")),
            kill_score: nonnegative_number(participant.get("kill_score")),
            ark_score: nonnegative_number(participant.get("flag_score")),
        })
        .collect()
}

fn map_highlights(document: &Document) -> Vec<ArkHighlight> {
    [
        "flag_score",
        "building_score",
        "gather_score",
        "killed_score",
        "be_killed_score",
        "healing_score",
    ]
    .into_iter()
    .filter_map(|category| {
        let value = nested_document(document, &["overview", category])?;
        Some(ArkHighlight {
            category: category.to_string(),
            alliance_value: nonnegative_number(value.get("alliance_score")),
            player_name: parse_string(nested_value(value, &["mvp", "player_name"])),
            player_value: nonnegative_number(nested_value(value, &["mvp", "score"])),
        })
    })
    .collect()
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

#[expect(clippy::float_cmp, reason = "Numeric boolean encodings must be exactly 0 or 1")]
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

pub(super) fn parse_string(value: Option<&Bson>) -> Option<String> {
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
            individual_results: None,
        };

        let mapped = map_match_record(&entry, 0);
        assert_eq!(mapped.winner_alliance_id, Some(8));
        assert_eq!(mapped.match_id, "m1");
    }

    #[test]
    fn participant_scores_preserve_decimals_and_distinguish_absence_from_nonparticipation() {
        let participants = map_participants(&doc! { "participants": [
            { "player_name": "Active", "individual_points": 250, "gather_score": 2.5 },
            { "player_name": "Absent", "individual_points": -1 },
            { "player_name": "Unknown" },
        ] });
        assert_eq!(participants[0].score, Some(250.0));
        assert_eq!(participants[0].provisions_score, Some(2.5));
        assert_eq!(participants[0].participated, Some(true));
        assert_eq!(participants[1].score, None);
        assert_eq!(participants[1].participated, Some(false));
        assert_eq!(participants[2].participated, None);
    }

    #[test]
    fn maps_pairing_loss_points_without_confusing_wounded_units() {
        let individual = doc! {
            "overview": { "rank": 0 },
            "results": { "total_score": 123123, "severely_wounded": 2760854 },
            "pairings": [{ "battles": 323, "kill_points": 28554639, "severely_wounded": 10580340 }],
        };
        let overview = map_match_overview(Some(&individual));
        assert_eq!(overview.rank, None);
        assert_eq!(overview.score, Some(123123));
        assert_eq!(map_individual_results(Some(&individual)).severely_wounded, Some(2760854));
        assert_eq!(map_pairings(Some(&individual))[0].loss_points, Some(10580340));
    }

    #[test]
    fn missing_individual_mail_stays_unknown_and_does_not_invent_zeroes() {
        let overview = map_match_overview(None);
        assert_eq!(overview.score, None);
        assert!(map_pairings(None).is_empty());
        assert_eq!(map_individual_results(None).kill_score, None);
    }

    #[test]
    fn speedup_minutes_preserve_recorded_duration_and_missing_values() {
        for (value, expected) in
            [(Bson::Int64(125), Some(125)), (Bson::Int32(0), Some(0)), (Bson::Null, None)]
        {
            let individual = doc! { "results": { "speedups": value } };
            assert_eq!(map_individual_results(Some(&individual)).speedups_minutes, expected);
        }
        assert_eq!(map_individual_results(Some(&doc! {})).speedups_minutes, None);
        assert_eq!(map_individual_results(None).speedups_minutes, None);
    }

    #[test]
    fn older_regular_mail_does_not_guess_golden_or_silver_without_a_session() {
        assert_eq!(
            league_from_mail(&doc! { "body": { "battle_type": "Egypt" } }),
            ArkLeague::Unknown
        );
        assert_eq!(
            league_from_mail(&doc! { "body": { "battle_type": "EgyptLeague" } }),
            ArkLeague::Osiris
        );
    }
}
