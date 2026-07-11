use core_bson::{
    nested_array, nested_bool, nested_document, nested_f64, nested_i64, nested_string,
};
use mongodb::bson::{Bson, Document, doc};

use super::types::{
    DuelBattle2DetailAlliance, DuelBattle2DetailBattleResult, DuelBattle2DetailBattleResults,
    DuelBattle2DetailBuff, DuelBattle2DetailCommander, DuelBattle2DetailCommanderSkill,
    DuelBattle2DetailItem, DuelBattle2DetailMetadata, DuelBattle2DetailPlayer,
};

pub(super) fn build_duelbattle2_detail_filter(duel_id: i64) -> Document {
    doc! { "sender.duel.team_id": duel_id }
}

pub(super) fn build_duelbattle2_detail_projection() -> Document {
    let mut projection = Document::new();
    projection.insert("_id", 0);

    for field in [
        "metadata.mail_id",
        "metadata.mail_time",
        "sender.player_id",
        "sender.player_name",
        "sender.avatar_url",
        "sender.frame_url",
        "sender.alliance.abbreviation",
        "sender.primary_commander.id",
        "sender.primary_commander.awakened",
        "sender.primary_commander.level",
        "sender.primary_commander.skills.id",
        "sender.primary_commander.skills.level",
        "sender.secondary_commander.id",
        "sender.secondary_commander.awakened",
        "sender.secondary_commander.level",
        "sender.secondary_commander.skills.id",
        "sender.secondary_commander.skills.level",
        "sender.buffs.id",
        "sender.buffs.value",
        "opponent.player_id",
        "opponent.player_name",
        "opponent.avatar_url",
        "opponent.frame_url",
        "opponent.alliance.abbreviation",
        "opponent.primary_commander.id",
        "opponent.primary_commander.awakened",
        "opponent.primary_commander.level",
        "opponent.primary_commander.skills.id",
        "opponent.primary_commander.skills.level",
        "opponent.secondary_commander.id",
        "opponent.secondary_commander.awakened",
        "opponent.secondary_commander.level",
        "opponent.secondary_commander.skills.id",
        "opponent.secondary_commander.skills.level",
        "opponent.buffs.id",
        "opponent.buffs.value",
        "battle_results.sender.win",
        "battle_results.sender.kill_points",
        "battle_results.sender.power",
        "battle_results.sender.units",
        "battle_results.sender.slightly_wounded",
        "battle_results.sender.severely_wounded",
        "battle_results.sender.dead",
        "battle_results.sender.heal",
        "battle_results.opponent.win",
        "battle_results.opponent.kill_points",
        "battle_results.opponent.power",
        "battle_results.opponent.units",
        "battle_results.opponent.slightly_wounded",
        "battle_results.opponent.severely_wounded",
        "battle_results.opponent.dead",
        "battle_results.opponent.heal",
    ] {
        projection.insert(field, 1);
    }

    projection
}

pub(super) fn map_duelbattle2_detail_document(
    document: &Document,
) -> Option<DuelBattle2DetailItem> {
    let metadata_document = nested_document(document, &["metadata"])?;
    let sender_document = nested_document(document, &["sender"])?;
    let opponent_document = nested_document(document, &["opponent"])?;
    let battle_results_document = nested_document(document, &["battle_results"]);

    Some(DuelBattle2DetailItem {
        metadata: map_detail_metadata(metadata_document)?,
        sender: map_detail_player(sender_document),
        opponent: map_detail_player(opponent_document),
        battle_results: map_detail_battle_results(battle_results_document),
    })
}

fn map_detail_metadata(document: &Document) -> Option<DuelBattle2DetailMetadata> {
    Some(DuelBattle2DetailMetadata {
        mail_id: nested_string(document, &["mail_id"])?,
        mail_time: nested_i64(document, &["mail_time"])?,
    })
}

fn map_detail_player(document: &Document) -> DuelBattle2DetailPlayer {
    DuelBattle2DetailPlayer {
        player_id: nested_i64(document, &["player_id"]).unwrap_or(0),
        player_name: nested_string(document, &["player_name"]).unwrap_or_default(),
        avatar_url: nested_string(document, &["avatar_url"]),
        frame_url: nested_string(document, &["frame_url"]),
        alliance: DuelBattle2DetailAlliance {
            abbreviation: nested_string(document, &["alliance", "abbreviation"])
                .unwrap_or_default(),
        },
        primary_commander: map_detail_commander(nested_document(document, &["primary_commander"])),
        secondary_commander: map_detail_commander(nested_document(
            document,
            &["secondary_commander"],
        )),
        buffs: map_detail_buffs(document),
    }
}

fn map_detail_commander(document: Option<&Document>) -> DuelBattle2DetailCommander {
    let Some(document) = document else {
        return DuelBattle2DetailCommander { id: 0, awakened: None, level: 0, skills: Vec::new() };
    };

    DuelBattle2DetailCommander {
        id: nested_i64(document, &["id"]).unwrap_or(0),
        awakened: nested_bool(document, &["awakened"]),
        level: nested_i64(document, &["level"]).unwrap_or(0),
        skills: map_detail_skills(document),
    }
}

fn map_detail_skills(document: &Document) -> Vec<DuelBattle2DetailCommanderSkill> {
    let Some(skills) = nested_array(document, &["skills"]) else {
        return Vec::new();
    };

    skills
        .iter()
        .filter_map(Bson::as_document)
        .map(|skill| DuelBattle2DetailCommanderSkill {
            id: nested_i64(skill, &["id"]).unwrap_or(0),
            level: nested_i64(skill, &["level"]).unwrap_or(0),
        })
        .collect()
}

fn map_detail_buffs(document: &Document) -> Vec<DuelBattle2DetailBuff> {
    let Some(buffs) = nested_array(document, &["buffs"]) else {
        return Vec::new();
    };

    buffs
        .iter()
        .filter_map(|value| {
            let Bson::Document(document) = value else {
                return None;
            };

            Some(DuelBattle2DetailBuff {
                id: nested_i64(document, &["id"]).unwrap_or(0),
                value: nested_f64(document, &["value"]).unwrap_or(0.0),
            })
        })
        .collect()
}

fn map_detail_battle_results(document: Option<&Document>) -> DuelBattle2DetailBattleResults {
    let Some(document) = document else {
        return DuelBattle2DetailBattleResults {
            sender: default_detail_battle_result(),
            opponent: default_detail_battle_result(),
        };
    };

    DuelBattle2DetailBattleResults {
        sender: map_detail_battle_result(nested_document(document, &["sender"])),
        opponent: map_detail_battle_result(nested_document(document, &["opponent"])),
    }
}

fn map_detail_battle_result(document: Option<&Document>) -> DuelBattle2DetailBattleResult {
    let Some(document) = document else {
        return default_detail_battle_result();
    };

    DuelBattle2DetailBattleResult {
        win: nested_bool(document, &["win"]).unwrap_or(false),
        kill_points: nested_i64(document, &["kill_points"]).unwrap_or(0),
        power: nested_i64(document, &["power"]).unwrap_or(0),
        units: nested_i64(document, &["units"]).unwrap_or(0),
        slightly_wounded: nested_i64(document, &["slightly_wounded"]).unwrap_or(0),
        severely_wounded: nested_i64(document, &["severely_wounded"]).unwrap_or(0),
        dead: nested_i64(document, &["dead"]).unwrap_or(0),
        heal: nested_i64(document, &["heal"]).unwrap_or(0),
    }
}

fn default_detail_battle_result() -> DuelBattle2DetailBattleResult {
    DuelBattle2DetailBattleResult {
        win: false,
        kill_points: 0,
        power: 0,
        units: 0,
        slightly_wounded: 0,
        severely_wounded: 0,
        dead: 0,
        heal: 0,
    }
}

#[cfg(test)]
mod tests {
    use mongodb::bson::{Bson, doc};

    use super::{
        build_duelbattle2_detail_filter, build_duelbattle2_detail_projection,
        map_duelbattle2_detail_document,
    };

    #[test]
    fn builds_detail_filter_from_duel_id() {
        let filter = build_duelbattle2_detail_filter(42);
        assert_eq!(filter.get_i64("sender.duel.team_id").ok(), Some(42));
    }

    #[test]
    fn includes_required_fields_in_detail_projection() {
        let projection = build_duelbattle2_detail_projection();
        assert_eq!(projection.get_i32("metadata.mail_id").ok(), Some(1));
        assert_eq!(projection.get_i32("sender.primary_commander.id").ok(), Some(1));
        assert_eq!(projection.get_i32("sender.primary_commander.awakened").ok(), Some(1));
        assert_eq!(projection.get_i32("sender.primary_commander.skills.id").ok(), Some(1));
        assert_eq!(projection.get_i32("battle_results.opponent.kill_points").ok(), Some(1));
    }

    #[test]
    fn maps_detail_document_when_required_fields_are_present() {
        let document = doc! {
            "metadata": {
                "mail_id": "mail-1",
                "mail_time": 123_i64,
            },
            "sender": {
                "player_id": 1_i64,
                "player_name": "Alpha",
                "avatar_url": "https://cdn/avatar.png",
                "frame_url": Bson::Null,
                "alliance": {
                    "abbreviation": "AAA",
                },
                "primary_commander": {
                    "id": 10_i64,
                    "awakened": true,
                    "level": 60_i64,
                    "skills": [
                        { "id": 101_i64, "level": 5_i64 },
                    ],
                },
                "secondary_commander": {
                    "id": 11_i64,
                    "level": 50_i64,
                    "skills": [
                        { "id": 201_i64, "level": 4_i64 },
                    ],
                },
                "buffs": [
                    { "id": 2001_i64, "value": 1.25 },
                    { "id": 2002_i64, "value": 300_i64 },
                ],
            },
            "opponent": {
                "player_id": 2_i64,
                "player_name": "Bravo",
                "alliance": {
                    "abbreviation": "BBB",
                },
                "primary_commander": {
                    "id": 12_i64,
                    "level": 60_i64,
                    "skills": [
                        { "id": 301_i64, "level": 5_i64 },
                    ],
                },
                "buffs": [],
            },
            "battle_results": {
                "sender": {
                    "win": true,
                    "kill_points": 150_i64,
                    "power": 200_i64,
                    "units": 300_i64,
                    "slightly_wounded": 30_i64,
                    "severely_wounded": 20_i64,
                    "dead": 10_i64,
                    "heal": 5_i64,
                },
                "opponent": {
                    "win": false,
                    "kill_points": 120_i64,
                    "power": 180_i64,
                    "units": 260_i64,
                    "slightly_wounded": 25_i64,
                    "severely_wounded": 15_i64,
                    "dead": 8_i64,
                    "heal": 4_i64,
                },
            },
        };

        let mapped = map_duelbattle2_detail_document(&document).expect("detail should map");
        assert_eq!(mapped.metadata.mail_id, "mail-1");
        assert_eq!(mapped.metadata.mail_time, 123);
        assert_eq!(mapped.sender.player_name, "Alpha");
        assert_eq!(mapped.sender.frame_url, None);
        assert_eq!(mapped.sender.primary_commander.id, 10);
        assert_eq!(mapped.sender.primary_commander.awakened, Some(true));
        assert_eq!(mapped.sender.primary_commander.skills[0].id, 101);
        assert_eq!(mapped.sender.buffs.len(), 2);
        assert_eq!(mapped.opponent.secondary_commander.id, 0);
        assert!(mapped.battle_results.sender.win);
        assert_eq!(mapped.battle_results.opponent.kill_points, 120);
    }

    #[test]
    fn skips_detail_document_without_required_metadata_fields() {
        let missing_mail_id = doc! {
            "metadata": {
                "mail_time": 123_i64,
            },
            "sender": {},
            "opponent": {},
        };

        assert!(map_duelbattle2_detail_document(&missing_mail_id).is_none());
    }
}
