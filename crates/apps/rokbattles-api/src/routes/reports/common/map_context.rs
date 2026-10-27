use std::collections::BTreeSet;

use futures::TryStreamExt;
use mongodb::bson::{Document, doc};
use rokbattles_bson::{nested_bool, nested_i64, nested_str};

use crate::{db::ReportsStore, error::ApiError};

const BANNER_BASE: &str = "https://cdn.rokbattles.com/game/sprites/";

#[derive(Debug, Clone, Copy)]
pub(crate) enum BattleMapContext {
    Home { kingdom: Option<i64> },
    RadiantRivalry { kingdom: Option<i64> },
    Ark { kingdom: Option<i64>, kind: Option<&'static str> },
    Strife { kingdom: Option<i64>, schema: Option<i64> },
    Kvk { server_id: i64, time: i64 },
    Unknown,
}

impl BattleMapContext {
    pub(crate) fn from_server_id(server_id: Option<i64>, time: i64) -> Self {
        match (server_id.filter(|id| *id > 0), report_time_millis(time)) {
            (Some(server_id), Some(time)) => Self::Kvk { server_id, time },
            _ => Self::Unknown,
        }
    }

    pub(crate) fn from_document(document: &Document, time: i64) -> Self {
        let kingdom = nested_i64(document, &["sender", "kingdom_id"])
            .filter(|id| *id > 0)
            .and_then(|id| id.checked_add(1000));

        if nested_str(document, &["sender", "supreme_strife", "battle_id"])
            .is_some_and(|id| !id.is_empty())
            && nested_i64(document, &["sender", "supreme_strife", "team_id"])
                .is_some_and(|id| id > 0)
        {
            let schema = nested_i64(document, &["metadata", "schema"]);
            return Self::Strife { kingdom, schema };
        }

        if nested_str(document, &["metadata", "mail_role"]) == Some("dungeon") {
            let kingdom = nested_i64(document, &["sender", "kingdom_id"]).filter(|id| *id > 0);
            let kind = nested_str(document, &["sender", "session"]).and_then(ark_kind);
            return Self::Ark { kingdom, kind };
        }

        if let Some(server_id) = radiant_rivalry_server_id(document) {
            return Self::RadiantRivalry { kingdom: server_id.checked_add(1000) };
        }

        match nested_bool(document, &["metadata", "kvk"]) {
            Some(false) => Self::Home { kingdom },
            Some(true) => Self::from_server_id(nested_i64(document, &["metadata", "gs_id"]), time),
            None => Self::Unknown,
        }
    }

    pub(crate) fn prokingdoms_url(self) -> Option<String> {
        let Self::Kvk { server_id, time } = self else {
            return None;
        };

        Some(format!("https://prokingdoms.com/kvk/{server_id}/{}", time / 1000))
    }

    pub(crate) fn resolve(self, snapshots: &[Document]) -> (String, Option<String>) {
        match self {
            Self::Home { kingdom } => {
                (kingdom_label(kingdom, "Home"), banner_url("img_ServerBanner1.png"))
            }
            Self::RadiantRivalry { kingdom } => {
                (kingdom_label(kingdom, "Home (RR)"), banner_url("img_ServerBanner1.png"))
            }
            Self::Ark { kingdom, kind } => {
                let label = kind.map_or_else(|| "Ark".into(), |kind| format!("Ark ({kind})"));
                let banner = match kind {
                    Some("GL") => banner_url("img_MailTypeBG_GVGClassification1.png"),
                    Some("SL") => banner_url("img_MailTypeBG_GVGClassification2.png"),
                    Some("OL") => banner_url("img_MailTypeBG_Egyptianleague.png"),
                    _ => None,
                };
                (kingdom_label(kingdom, &label), banner)
            }
            Self::Strife { kingdom, schema } => {
                let filename = match schema {
                    Some(20002) => "img_MailTypeBG_MailTopKvkBattleReport.png",
                    _ => "img_MailTypeBG_TopKVK.png",
                };
                (kingdom_label(kingdom, "Strife"), banner_url(filename))
            }
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

fn radiant_rivalry_server_id(document: &Document) -> Option<i64> {
    if nested_str(document, &["metadata", "mail_role"]) != Some("gsmp")
        || nested_i64(document, &["metadata", "schema"]) != Some(10)
    {
        return None;
    }

    let server_id = nested_i64(document, &["metadata", "gs_id"]).filter(|id| *id > 0)?;
    let is_foreign = |id: i64| id > 0 && id != server_id;

    let foreign_sender = nested_i64(document, &["sender", "kingdom_id"]).is_some_and(is_foreign);

    let foreign_opponent = document.get_array("opponents").is_ok_and(|opponents| {
        opponents
            .iter()
            .filter_map(|opponent| opponent.as_document())
            .any(|opponent| nested_i64(opponent, &["kingdom_id"]).is_some_and(is_foreign))
    });

    (foreign_sender || foreign_opponent).then_some(server_id)
}

fn kingdom_label(kingdom: Option<i64>, label: &str) -> String {
    kingdom.map_or_else(|| label.into(), |kingdom| format!("#{kingdom} \u{00B7} {label}"))
}

fn ark_kind(session: &str) -> Option<&'static str> {
    let parameter = |name| {
        session
            .split('&')
            .filter_map(|part| part.split_once('='))
            .find_map(|(key, value)| (key == name).then_some(value))
    };
    match (parameter("mode"), parameter("submode")) {
        (_, Some("SilverEgypt")) => Some("SL"),
        (Some("abl"), _) => Some("OL"),
        (Some("ab"), Some("")) => Some("GL"),
        (Some("abp"), Some("gvgn")) => Some("P"),
        (Some("abp"), Some("DiyEgypt")) => Some("C"),
        _ => None,
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

pub(crate) async fn load_map_snapshots(
    store: &ReportsStore,
    contexts: impl Iterator<Item = BattleMapContext>,
) -> Result<Vec<Document>, ApiError> {
    let ids: BTreeSet<_> = contexts
        .filter_map(|context| match context {
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
    Ok(snapshots)
}

fn public_mapcode(snapshot: &Document) -> Option<String> {
    let id = nested_i64(snapshot, &["Id"])?;
    if (10_001..=11_000).contains(&id) {
        let rush_order = id - 10_010;
        return Some(format!("#SD{}", 1000 + rush_order));
    }
    let order = nested_i64(snapshot, &["Order"]).filter(|order| *order > 0)?;
    let mode = nested_i64(snapshot, &["Mode"]).filter(|mode| *mode > 0)?;
    let prefix = if mode > 8 { 'C' } else { 'S' };
    Some(format!("#{prefix}{order}"))
}

fn banner_url(filename: &str) -> Option<String> {
    Some(format!("{BANNER_BASE}{filename}"))
}

fn map_banner(map: &str) -> Option<String> {
    let filename = match map {
        "LostLand_Map_1_2_v2" => "img_ServerBanner2.png",
        "LostLand_Map_2_2_v2" => "img_ServerBanner2_1.png",
        "LostLand_Map_3_v2" => "img_ServerBanner4.png",
        "LostLand_Map_3_2_v2" | "LostLand_Map_19_2" => "img_ServerBanner19.png",
        "LostLand_Map_4_2_v2" => "img_ServerBanner5.png",
        "LostLand_Map_5_v2" => "img_ServerBanner6.png",
        "LostLand_Map_9_2_v2" => "img_ServerBanner10.png",
        "LostLand_Map_10" => "img_ServerBanner12.png",
        "LostLand_Map_11_2_v2" => "img_ServerBanner11.png",
        "LostLand_Map_12_2_v2" => "img_ServerBanner13.png",
        "LostLand_Map_14_v2" => "img_ServerBanner14.png",
        "LostLand_Map_15_v2" => "img_ServerBanner15.png",
        "LostLand_Map_16_2_v2" => "img_ServerBanner16.png",
        "LostLand_Map_18_v2" => "img_ServerBanner18.png",
        "LostLand_Map_20_v2" => "img_ServerBanner20.png",
        "LostLand_Map_21_v2" => "img_ServerBannerS21.png",
        _ => return None,
    };
    banner_url(filename)
}

#[cfg(test)]
mod tests {
    use mongodb::bson::Bson;

    use super::*;

    fn snapshot() -> Document {
        doc! {
            "Id": 16053, "Order": 13249, "Mode": 16,
            "OpenTime": 1000, "CloseTime": 2000, "EndTime": 1900,
            "MapName": "LostLand_Map_4_2_v2",
        }
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
    fn prokingdoms_url_normalizes_mail_timestamp_units_to_seconds() {
        for time in
            [1_766_182_379, 1_766_182_379_846, 1_766_182_379_846_826, 1_766_182_379_846_826_000]
        {
            assert_eq!(
                BattleMapContext::from_server_id(Some(15790), time).prokingdoms_url().as_deref(),
                Some("https://prokingdoms.com/kvk/15790/1766182379")
            );
        }
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
            (1000, "#C13249"),
            (1950, "#C13249"),
            (2000, "Unknown"),
            (3000, "#C13300"),
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
        for (mode, expected) in [(2, "#S13249"), (4, "#S13249"), (8, "#S13249"), (16, "#C13249")] {
            info.insert("Mode", mode);
            assert_eq!(public_mapcode(&info).as_deref(), Some(expected));
        }
        info.insert("Id", 10011);
        info.remove("Order");
        info.remove("Mode");
        assert_eq!(public_mapcode(&info).as_deref(), Some("#SD1001"));
    }

    #[test]
    fn s3_banners_depend_on_the_map_not_the_season_or_mode() {
        let mut info = snapshot();
        info.insert("Mode", 8);
        info.insert("Season", 3);
        let context = BattleMapContext::Kvk { server_id: 16053, time: 1500 };
        for (map, filename) in [
            ("LostLand_Map_3_v2", "img_ServerBanner4.png"),
            ("LostLand_Map_3_2_v2", "img_ServerBanner19.png"),
        ] {
            info.insert("MapName", map);
            assert_eq!(context.resolve(&[info.clone()]), ("#S13249".into(), banner_url(filename)));
        }
    }

    #[test]
    fn strife_of_the_eight_and_shifting_gears_resolve_to_sprite_banners() {
        let context = BattleMapContext::Kvk { server_id: 16053, time: 1500 };
        for (map, url) in [
            ("LostLand_Map_5_v2", "https://cdn.rokbattles.com/game/sprites/img_ServerBanner6.png"),
            (
                "LostLand_Map_18_v2",
                "https://cdn.rokbattles.com/game/sprites/img_ServerBanner18.png",
            ),
        ] {
            let mut info = snapshot();
            info.insert("MapName", map);
            assert_eq!(context.resolve(&[info]), ("#C13249".into(), Some(url.into())));
        }
    }

    #[test]
    fn known_kvk_with_unsupported_or_missing_map_keeps_code_without_banner() {
        let context = BattleMapContext::Kvk { server_id: 16053, time: 1500 };
        let mut info = snapshot();
        info.insert("MapName", "unsupported_map");
        assert_eq!(context.resolve(&[info.clone()]), ("#C13249".into(), None));
        info.remove("MapName");
        assert_eq!(context.resolve(&[info]), ("#C13249".into(), None));
    }

    #[test]
    fn ark_and_strife_override_kvk_and_home_flags() {
        for kvk in [true, false] {
            let mut report = doc! { "metadata": { "kvk": kvk, "mail_role": "dungeon" } };
            assert_eq!(
                BattleMapContext::from_document(&report, 1500).resolve(&[]),
                ("Ark".into(), None)
            );
            report.insert(
                "sender",
                doc! { "supreme_strife": { "battle_id": "match", "team_id": 1 } },
            );
            assert_eq!(
                BattleMapContext::from_document(&report, 1500).resolve(&[]),
                ("Strife".into(), banner_url("img_MailTypeBG_TopKVK.png"))
            );
        }
    }

    #[test]
    fn only_explicit_home_reports_get_the_preparation_banner() {
        assert_eq!(
            BattleMapContext::from_document(&doc! { "metadata": { "kvk": false } }, 1500)
                .resolve(&[]),
            ("Home".into(), banner_url("img_ServerBanner1.png"))
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
    fn battle_kvk_never_falls_back_to_envelope_server() {
        for gs_id in [None, Some(Bson::Null), Some(Bson::Int32(0)), Some(Bson::Int32(-1))] {
            let mut metadata = doc! { "kvk": true, "server_id": 16053 };
            if let Some(gs_id) = gs_id {
                metadata.insert("gs_id", gs_id);
            }
            let context = BattleMapContext::from_document(&doc! { "metadata": metadata }, 1);

            assert_eq!(context.resolve(&[snapshot()]), ("Unknown".into(), None));
            assert!(context.prokingdoms_url().is_none());
        }
    }

    #[test]
    fn strife_uses_sender_kingdom_and_schema_banner() {
        for (schema, filename) in [
            (20001, "img_MailTypeBG_TopKVK.png"),
            (20002, "img_MailTypeBG_MailTopKvkBattleReport.png"),
            (0, "img_MailTypeBG_TopKVK.png"),
        ] {
            let report = doc! {
                "metadata": {
                    "kvk": true, "mail_role": "dungeon", "gs_id": 99, "schema": schema,
                },
                "sender": {
                    "kingdom_id": 1804,
                    "supreme_strife": { "battle_id": "match", "team_id": 1 },
                },
            };
            assert_eq!(
                BattleMapContext::from_document(&report, 1500).resolve(&[]),
                ("#2804 \u{00B7} Strife".into(), banner_url(filename))
            );
        }
    }

    #[test]
    fn home_uses_sender_kingdom_instead_of_report_server() {
        let report = doc! {
            "metadata": { "kvk": false, "gs_id": 99 },
            "sender": { "kingdom_id": 1804 },
        };
        assert_eq!(
            BattleMapContext::from_document(&report, 1500).resolve(&[]),
            ("#2804 \u{00B7} Home".into(), banner_url("img_ServerBanner1.png"))
        );
    }

    #[test]
    fn radiant_rivalry_overrides_home_and_kvk_independently_of_ll_script_schema() {
        for (sender, opponent) in [(3028, 3030), (3030, 3028)] {
            for kvk in [true, false] {
                for ll_schema in [0, 224] {
                    let report = doc! {
                        "metadata": {
                            "mail_role": "gsmp", "schema": 10, "gs_id": 3030, "server_id": 3028,
                            "kvk": kvk, "ll_script_schema": ll_schema,
                        },
                        "sender": { "kingdom_id": sender },
                        "opponents": [{ "kingdom_id": 0 }, { "kingdom_id": opponent }],
                    };

                    assert_eq!(
                        BattleMapContext::from_document(&report, 1500).resolve(&[]),
                        ("#4030 · Home (RR)".into(), banner_url("img_ServerBanner1.png"))
                    );
                }
            }
        }
    }

    #[test]
    fn home_combat_with_local_npc_or_unknown_opponents_stays_home() {
        for opponent in [
            doc! { "kingdom_id": 3030 },
            doc! { "kingdom_id": 0 },
            doc! { "kingdom_id": -1 },
            doc! {},
            doc! { "kingdom_id": "unknown" },
        ] {
            let report = doc! {
                "metadata": { "mail_role": "gsmp", "schema": 10, "gs_id": 3030, "server_id": 3028, "kvk": false },
                "sender": { "kingdom_id": 3030 },
                "opponents": [opponent],
            };

            assert_eq!(
                BattleMapContext::from_document(&report, 1500).resolve(&[]),
                ("#4030 · Home".into(), banner_url("img_ServerBanner1.png"))
            );
        }
    }

    #[test]
    fn radiant_rivalry_requires_home_schema_role_and_known_battle_server() {
        for (role, schema, server) in
            [("gsmp", 320, 16053), ("gs", 10, 16053), ("gsmp", 10, 0), ("gsmp", 10, -1)]
        {
            let report = doc! {
                "metadata": { "mail_role": role, "schema": schema, "gs_id": server, "kvk": true },
                "sender": { "kingdom_id": 3028 },
                "opponents": [{ "kingdom_id": 3030 }],
            };

            let expected = if server > 0 { "#C13249" } else { "Unknown" };

            assert_eq!(
                BattleMapContext::from_document(&report, 1).resolve(&[snapshot()]).0,
                expected
            );
        }

        let report = doc! {
            "metadata": { "mail_role": "gsmp", "schema": 10, "kvk": true },
            "sender": { "kingdom_id": 3028 },
        };

        assert_eq!(BattleMapContext::from_document(&report, 1).resolve(&[]).0, "Unknown");
    }

    #[test]
    fn strife_takes_precedence_over_radiant_rivalry_heuristic() {
        let report = doc! {
            "metadata": { "mail_role": "gsmp", "schema": 10, "gs_id": 3030, "server_id": 3028, "kvk": true },
            "sender": {
                "kingdom_id": 3028,
                "supreme_strife": { "battle_id": "match", "team_id": 1 },
            },
        };

        assert_eq!(BattleMapContext::from_document(&report, 1500).resolve(&[]).0, "#4028 · Strife");
    }

    #[test]
    fn ark_labels_match_types_and_only_banners_selected_modes() {
        for (session, kind, filename) in [
            (
                "cfg_id=4&id=3538&mode=ab&role_id=113&submode=",
                "GL",
                Some("img_MailTypeBG_GVGClassification1.png"),
            ),
            ("submode=SilverEgypt&mode=ab", "SL", Some("img_MailTypeBG_GVGClassification2.png")),
            ("mode=abl&submode=", "OL", Some("img_MailTypeBG_Egyptianleague.png")),
            ("mode=abp&submode=gvgn", "P", None),
            ("mode=abp&submode=DiyEgypt", "C", None),
        ] {
            let report = doc! {
                "metadata": { "mail_role": "dungeon", "gs_id": 1804 },
                "sender": { "kingdom_id": 117, "session": session },
            };
            assert_eq!(
                BattleMapContext::from_document(&report, 1500).resolve(&[]),
                (format!("#117 \u{00B7} Ark ({kind})"), filename.and_then(banner_url))
            );
        }
    }

    #[test]
    fn missing_kingdom_or_unrecognized_ark_session_does_not_invent_values() {
        for kingdom in [0, -1] {
            let report = doc! {
                "metadata": { "mail_role": "dungeon" },
                "sender": { "kingdom_id": kingdom, "session": "mode=abp&submode=unknown" },
            };
            assert_eq!(
                BattleMapContext::from_document(&report, 1500).resolve(&[]),
                ("Ark".into(), None)
            );
        }
        assert_eq!(ark_kind("othermode=ab&submode="), None);
        assert_eq!(ark_kind("mode=ab"), None);
    }
}
