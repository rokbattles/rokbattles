use std::collections::BTreeSet;

use futures::TryStreamExt;
use mongodb::bson::{Document, doc};
use rokbattles_bson::{nested_bool, nested_i64, nested_str};

use super::types::ReportRowWithCursor;
use crate::{db::ReportsStore, error::ApiError};

const BANNER_BASE: &str = "https://cdn.rokbattles.com/game/banners/";

#[derive(Debug, Clone, Copy)]
pub(crate) enum BattleMapContext {
    Home,
    Ark,
    Strife,
    Kvk { server_id: i64, time: i64 },
    Unknown,
}

impl BattleMapContext {
    pub(super) fn from_document(document: &Document, time: i64) -> Self {
        if nested_str(document, &["sender", "supreme_strife", "battle_id"])
            .is_some_and(|id| !id.is_empty())
            && nested_i64(document, &["sender", "supreme_strife", "team_id"])
                .is_some_and(|id| id > 0)
        {
            return Self::Strife;
        }
        if nested_str(document, &["metadata", "mail_role"]) == Some("dungeon") {
            return Self::Ark;
        }
        match nested_bool(document, &["metadata", "kvk"]) {
            Some(false) => Self::Home,
            Some(true) => {
                let Some(time) = report_time_millis(time) else {
                    return Self::Unknown;
                };
                nested_i64(document, &["metadata", "server_id"])
                    .filter(|id| *id > 0)
                    .map_or(Self::Unknown, |server_id| Self::Kvk { server_id, time })
            }
            None => Self::Unknown,
        }
    }

    fn resolve(self, snapshots: &[Document]) -> (String, Option<String>) {
        match self {
            Self::Home => ("Home".into(), banner_url("preparation_season01_cover.png")),
            Self::Ark => ("AOO".into(), None),
            Self::Strife => ("Strife".into(), None),
            Self::Unknown => ("Unknown".into(), None),
            Self::Kvk { server_id, time } => {
                let mut matches = snapshots.iter().filter(|snapshot| {
                    nested_i64(snapshot, &["Id"]) == Some(server_id)
                        && nested_i64(snapshot, &["OpenTime"])
                            .is_some_and(|open| open > 0 && open <= time)
                        && nested_i64(snapshot, &["CloseTime"]).is_some_and(|close| time < close)
                });
                let Some(snapshot) = matches.next() else {
                    return ("Unknown".into(), None);
                };
                if matches.next().is_some() {
                    return ("Unknown".into(), None);
                }
                let Some(code) = public_mapcode(snapshot) else {
                    return ("Unknown".into(), None);
                };
                let banner = nested_str(snapshot, &["MapName"]).and_then(map_banner);
                (code, banner)
            }
        }
    }
}

fn report_time_millis(time: i64) -> Option<i64> {
    match time {
        ..=0 => None,
        100_000_000_000_000_000.. => Some(time / 1_000_000),
        100_000_000_000_000.. => Some(time / 1000),
        1_000_000_000_000.. => Some(time),
        _ => time.checked_mul(1000),
    }
}

pub(super) async fn enrich_map_context(
    store: &ReportsStore,
    rows: &mut [ReportRowWithCursor],
) -> Result<(), ApiError> {
    let ids: BTreeSet<_> = rows
        .iter()
        .filter_map(|row| match row.map_context {
            BattleMapContext::Kvk { server_id, .. } => Some(server_id),
            _ => None,
        })
        .collect();
    let snapshots: Vec<Document> = if ids.is_empty() {
        Vec::new()
    } else {
        store
            .lostland_collection()
            .find(doc! { "Id": { "$in": ids.into_iter().collect::<Vec<_>>() } })
            .projection(doc! {
                "_id": 0, "Id": 1, "OpenTime": 1, "CloseTime": 1,
                "Mode": 1, "Order": 1, "MapName": 1,
            })
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?
            .try_collect()
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?
    };
    for row in rows {
        (row.item.kvk_mapcode, row.item.kvk_banner) = row.map_context.resolve(&snapshots);
    }
    Ok(())
}

fn public_mapcode(snapshot: &Document) -> Option<String> {
    let id = nested_i64(snapshot, &["Id"])?;
    if (10_001..=11_000).contains(&id) {
        let rush_order = id - 10_010;
        return Some(format!("SD{}", 1000 + rush_order));
    }
    let order = nested_i64(snapshot, &["Order"]).filter(|order| *order > 0)?;
    let mode = nested_i64(snapshot, &["Mode"]).filter(|mode| *mode > 0)?;
    let prefix = if mode > 8 { 'C' } else { 'S' };
    Some(format!("{prefix}{order}"))
}

fn banner_url(filename: &str) -> Option<String> {
    Some(format!("{BANNER_BASE}{filename}"))
}

fn map_banner(map: &str) -> Option<String> {
    let filename = match map {
        "LostLand_Map_1_2_v2" => "S1Endless-War.png",
        "LostLand_Map_2_2_v2" => "S2Distant-Journey.png",
        "LostLand_Map_3_v2" => "S3Light-and-Darkness.png",
        "LostLand_Map_3_2_v2" | "LostLand_Map_19_2" => "s19king_of_all_britain_cover.png",
        "LostLand_Map_4_2_v2" => "s4heroic_anthem_cover.png",
        "LostLand_Map_9_2_v2" => "s9king_of_the_nile_cover.png",
        "LostLand_Map_10" => "s10siege_of_orleans_cover.png",
        "LostLand_Map_11_2_v2" => "s11warriors_unbound_cover.png",
        "LostLand_Map_12_2_v2" => "s12storm_of_stratagems_cover.png",
        "LostLand_Map_14_v2" => "s14tides_of_war_cover.png",
        "LostLand_Map_15_v2" => "s15alliance_invictus_cover.png",
        "LostLand_Map_16_2_v2" => "s16keener_blades_cover.png",
        "LostLand_Map_20_v2" => "s20song_of_troy_cover.png",
        _ => return None,
    };
    banner_url(filename)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> Document {
        doc! {
            "Id": 16053, "Order": 13249, "Mode": 16,
            "OpenTime": 1000, "CloseTime": 2000, "EndTime": 1900,
            "MapName": "LostLand_Map_4_2_v2",
        }
    }

    #[test]
    fn converts_battle_seconds_to_lostland_milliseconds_without_changing_public_time() {
        use super::super::list_mapper::map_battle_list_document;
        let mut info = snapshot();
        info.insert("OpenTime", 1_787_875_200_000_i64);
        info.insert("CloseTime", 1_792_368_000_000_i64);
        let report = doc! {
            "metadata": { "mail_id": "test", "mail_time": 1_789_929_245_i64, "kvk": true, "server_id": 16053 },
            "timeline": { "start_timestamp": 1_789_929_193_i64 },
            "opponents": [{ "player_id": 1 }],
        };
        let row = map_battle_list_document(&report).expect("valid report");
        assert_eq!(row.map_context.resolve(&[info]).0, "C13249");
        assert_eq!(row.item.time_start, 1_789_929_193);
    }

    #[test]
    fn normalizes_mail_time_fallback_and_rejects_missing_times() {
        for time in
            [1_789_929_193, 1_789_929_193_000, 1_789_929_193_000_000, 1_789_929_193_000_000_000]
        {
            assert_eq!(report_time_millis(time), Some(1_789_929_193_000));
        }
        assert_eq!(report_time_millis(0), None);
        assert_eq!(report_time_millis(-1), None);
    }

    #[test]
    fn reused_server_matches_the_battle_time_and_excludes_close_time() {
        let first = snapshot();
        let mut second = snapshot();
        second.insert("OpenTime", 3000);
        second.insert("CloseTime", 4000);
        second.insert("Order", 13300);
        let snapshots = [first, second];
        for (time, expected) in [
            (999, "Unknown"),
            (1000, "C13249"),
            (1950, "C13249"),
            (2000, "Unknown"),
            (3000, "C13300"),
            (4000, "Unknown"),
        ] {
            assert_eq!(
                BattleMapContext::Kvk { server_id: 16053, time }.resolve(&snapshots).0,
                expected
            );
        }
    }

    #[test]
    fn missing_or_ambiguous_kvk_never_gets_a_banner() {
        let context = BattleMapContext::Kvk { server_id: 16053, time: 1500 };
        assert_eq!(context.resolve(&[]), ("Unknown".into(), None));
        assert_eq!(context.resolve(&[snapshot(), snapshot()]), ("Unknown".into(), None));
        let mut invalid = snapshot();
        invalid.insert("Order", 0);
        assert_eq!(context.resolve(&[invalid]), ("Unknown".into(), None));
    }

    #[test]
    fn public_prefix_uses_mode_and_rush_number_instead_of_internal_id() {
        let mut info = snapshot();
        for (mode, expected) in [(2, "S13249"), (4, "S13249"), (8, "S13249"), (16, "C13249")] {
            info.insert("Mode", mode);
            assert_eq!(public_mapcode(&info).as_deref(), Some(expected));
        }
        info.insert("Id", 10011);
        info.remove("Order");
        info.remove("Mode");
        assert_eq!(public_mapcode(&info).as_deref(), Some("SD1001"));
    }

    #[test]
    fn s3_banners_depend_on_the_map_not_the_season_or_mode() {
        let mut info = snapshot();
        info.insert("Mode", 8);
        info.insert("Season", 3);
        let context = BattleMapContext::Kvk { server_id: 16053, time: 1500 };
        for (map, filename) in [
            ("LostLand_Map_3_v2", "S3Light-and-Darkness.png"),
            ("LostLand_Map_3_2_v2", "s19king_of_all_britain_cover.png"),
        ] {
            info.insert("MapName", map);
            assert_eq!(context.resolve(&[info.clone()]), ("S13249".into(), banner_url(filename)));
        }
    }

    #[test]
    fn known_kvk_with_unsupported_or_missing_map_keeps_code_without_banner() {
        let context = BattleMapContext::Kvk { server_id: 16053, time: 1500 };
        let mut info = snapshot();
        info.insert("MapName", "LostLand_Map_5_v2");
        assert_eq!(context.resolve(&[info.clone()]), ("C13249".into(), None));
        info.remove("MapName");
        assert_eq!(context.resolve(&[info]), ("C13249".into(), None));
    }

    #[test]
    fn ark_and_strife_override_kvk_and_home_flags() {
        for kvk in [true, false] {
            let mut report = doc! { "metadata": { "kvk": kvk, "mail_role": "dungeon" } };
            assert_eq!(
                BattleMapContext::from_document(&report, 1500).resolve(&[]),
                ("AOO".into(), None)
            );
            report.insert(
                "sender",
                doc! { "supreme_strife": { "battle_id": "match", "team_id": 1 } },
            );
            assert_eq!(
                BattleMapContext::from_document(&report, 1500).resolve(&[]),
                ("Strife".into(), None)
            );
        }
    }

    #[test]
    fn only_explicit_home_reports_get_the_preparation_banner() {
        assert_eq!(
            BattleMapContext::from_document(&doc! { "metadata": { "kvk": false } }, 1500)
                .resolve(&[]),
            ("Home".into(), banner_url("preparation_season01_cover.png"))
        );
        assert_eq!(
            BattleMapContext::from_document(&doc! {}, 1500).resolve(&[]),
            ("Unknown".into(), None)
        );
        assert_eq!(
            BattleMapContext::from_document(&doc! { "metadata": { "kvk": true } }, 1500)
                .resolve(&[]),
            ("Unknown".into(), None)
        );
    }

    #[test]
    fn list_projection_and_serialization_keep_lookup_fields_private() {
        use super::super::list_mapper::{build_battle_list_projection, map_battle_list_document};
        let projection = build_battle_list_projection();
        for field in [
            "metadata.server_id",
            "metadata.kvk",
            "metadata.mail_role",
            "sender.supreme_strife.battle_id",
            "sender.supreme_strife.team_id",
        ] {
            assert_eq!(projection.get_i32(field), Ok(1));
        }
        let report = doc! {
            "metadata": { "mail_id": "test", "mail_time": 1, "kvk": true, "server_id": 16053 },
            "opponents": [{ "player_id": 1 }],
        };
        let mut row = map_battle_list_document(&report).expect("valid report");
        (row.item.kvk_mapcode, row.item.kvk_banner) = row.map_context.resolve(&[snapshot()]);
        let value = serde_json::to_value(row.item).expect("serializable item");
        assert_eq!(value["kvkMapcode"], "C13249");
        assert_eq!(value["kvkBanner"], format!("{BANNER_BASE}s4heroic_anthem_cover.png"));
        assert_eq!(value.as_object().expect("object").len(), 12);
        for private in ["serverId", "mapContext", "Order", "OpenTime", "CloseTime", "Mode"] {
            assert!(value.get(private).is_none(), "leaked {private}");
        }
    }
}
