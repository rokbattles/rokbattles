use core_bson::{
    nested_array, nested_bool, nested_document, nested_i64, nested_i64_exact, nested_str,
    nested_string,
};
use mongodb::bson::{Bson, Document, doc};

use super::{
    structure_override::resolve_avatar_override,
    types::{
        BattleReportAlliance, BattleReportArmament, BattleReportAttack, BattleReportAuxiliarySkill,
        BattleReportBattleResult, BattleReportBattleResults, BattleReportCastle,
        BattleReportCommander, BattleReportCommanderSet, BattleReportCommanderSkill,
        BattleReportDetail, BattleReportMetadata, BattleReportNpc, BattleReportOpponent,
        BattleReportPlayer, BattleReportRelic, BattleReportSummary, BattleReportSummaryEntry,
        BattleReportSupportSkill, BattleReportSupportSkills, BattleReportTimeline,
    },
};

pub(super) fn build_battle_detail_filter(report_id: &str) -> Document {
    doc! { "metadata.mail_id": report_id }
}

pub(super) fn build_battle_detail_projection() -> Document {
    let mut projection = Document::new();
    projection.insert("_id", 0);

    for field in [
        "metadata.mail_id",
        "metadata.mail_time",
        "metadata.mail_role",
        "metadata.kvk",
        "metadata.schema",
        "sender.player_id",
        "sender.player_name",
        "sender.alliance.abbreviation",
        "sender.avatar_url",
        "sender.frame_url",
        "sender.tracking_key",
        "sender.rally",
        "sender.alliance_building_id",
        "sender.character_type",
        "sender.is_turret",
        "sender.is_outpost",
        "sender.structure_id",
        "sender.castle.x",
        "sender.castle.y",
        "sender.app_uid",
        "sender.support_skills.enable",
        "sender.support_skills.skills.hero_id",
        "sender.support_skills.skills.skill_id",
        "sender.support_skills.skills.skill_level",
        "sender.auxiliary_skills.hero_id",
        "sender.auxiliary_skills.level",
        "sender.auxiliary_skills.skill_id",
        "sender.commanders.primary.id",
        "sender.commanders.primary.awakened",
        "sender.commanders.primary.level",
        "sender.commanders.primary.formation",
        "sender.commanders.primary.equipment",
        "sender.commanders.primary.relics.id",
        "sender.commanders.primary.skills.id",
        "sender.commanders.primary.skills.level",
        "sender.commanders.primary.armaments.affix",
        "sender.commanders.primary.armaments.buffs",
        "sender.commanders.secondary.id",
        "sender.commanders.secondary.awakened",
        "sender.commanders.secondary.level",
        "sender.commanders.secondary.skills.id",
        "sender.commanders.secondary.skills.level",
        "sender.commanders.secondary.equipment",
        "sender.commanders.secondary.relics.id",
        "sender.commanders.secondary.armaments.affix",
        "sender.commanders.secondary.armaments.buffs",
        "summary.sender.kill_points",
        "summary.sender.dead",
        "summary.sender.severely_wounded",
        "summary.sender.slightly_wounded",
        "summary.sender.remaining",
        "summary.sender.troop_units",
        "summary.opponent.kill_points",
        "summary.opponent.dead",
        "summary.opponent.severely_wounded",
        "summary.opponent.slightly_wounded",
        "summary.opponent.remaining",
        "summary.opponent.troop_units",
        "timeline.start_timestamp",
        "timeline.start_tick",
        "opponents.player_id",
        "opponents.start_tick",
        "opponents.end_tick",
        "opponents.attack.x",
        "opponents.attack.y",
        "opponents.npc.type",
        "opponents.npc.b_type",
        "opponents.player_name",
        "opponents.alliance.abbreviation",
        "opponents.avatar_url",
        "opponents.frame_url",
        "opponents.rally",
        "opponents.alliance_building_id",
        "opponents.character_type",
        "opponents.is_turret",
        "opponents.is_outpost",
        "opponents.structure_id",
        "opponents.castle.x",
        "opponents.castle.y",
        "opponents.app_uid",
        "opponents.tracking_key",
        "opponents.support_skills.enable",
        "opponents.support_skills.skills.hero_id",
        "opponents.support_skills.skills.skill_id",
        "opponents.support_skills.skills.skill_level",
        "opponents.auxiliary_skills.hero_id",
        "opponents.auxiliary_skills.level",
        "opponents.auxiliary_skills.skill_id",
        "opponents.commanders.primary.id",
        "opponents.commanders.primary.awakened",
        "opponents.commanders.primary.level",
        "opponents.commanders.primary.formation",
        "opponents.commanders.primary.equipment",
        "opponents.commanders.primary.relics.id",
        "opponents.commanders.primary.skills.id",
        "opponents.commanders.primary.skills.level",
        "opponents.commanders.primary.armaments.affix",
        "opponents.commanders.primary.armaments.buffs",
        "opponents.commanders.secondary.id",
        "opponents.commanders.secondary.awakened",
        "opponents.commanders.secondary.level",
        "opponents.commanders.secondary.skills.id",
        "opponents.commanders.secondary.skills.level",
        "opponents.commanders.secondary.equipment",
        "opponents.commanders.secondary.relics.id",
        "opponents.commanders.secondary.armaments.affix",
        "opponents.commanders.secondary.armaments.buffs",
        "opponents.battle_results.sender.reinforcements_join",
        "opponents.battle_results.sender.reinforcements_leave",
        "opponents.battle_results.sender.kill_points",
        "opponents.battle_results.sender.acclaim",
        "opponents.battle_results.sender.severely_wounded",
        "opponents.battle_results.sender.slightly_wounded",
        "opponents.battle_results.sender.remaining",
        "opponents.battle_results.sender.dead",
        "opponents.battle_results.sender.heal",
        "opponents.battle_results.sender.troop_units",
        "opponents.battle_results.sender.troop_units_max",
        "opponents.battle_results.sender.power",
        "opponents.battle_results.sender.attack_power",
        "opponents.battle_results.sender.skill_power",
        "opponents.battle_results.opponent.reinforcements_join",
        "opponents.battle_results.opponent.reinforcements_leave",
        "opponents.battle_results.opponent.kill_points",
        "opponents.battle_results.opponent.acclaim",
        "opponents.battle_results.opponent.severely_wounded",
        "opponents.battle_results.opponent.slightly_wounded",
        "opponents.battle_results.opponent.remaining",
        "opponents.battle_results.opponent.dead",
        "opponents.battle_results.opponent.heal",
        "opponents.battle_results.opponent.troop_units",
        "opponents.battle_results.opponent.troop_units_max",
        "opponents.battle_results.opponent.power",
        "opponents.battle_results.opponent.attack_power",
        "opponents.battle_results.opponent.skill_power",
    ] {
        projection.insert(field, 1);
    }

    projection
}

pub(super) fn map_battle_detail_document(document: &Document) -> Option<BattleReportDetail> {
    let report_schema = nested_i64_exact(document, &["metadata", "schema"]);
    Some(BattleReportDetail {
        metadata: map_detail_metadata(document)?,
        sender: map_detail_player(nested_document(document, &["sender"]), report_schema),
        summary: map_detail_summary(nested_document(document, &["summary"])),
        opponents: map_detail_opponents(document, report_schema),
        timeline: map_detail_timeline(nested_document(document, &["timeline"])),
    })
}

fn map_detail_metadata(document: &Document) -> Option<BattleReportMetadata> {
    Some(BattleReportMetadata {
        mail_id: nested_str(document, &["metadata", "mail_id"])?.to_string(),
        mail_time: nested_i64(document, &["metadata", "mail_time"])?,
        mail_role: nested_string(document, &["metadata", "mail_role"]),
        kvk: nested_bool(document, &["metadata", "kvk"]),
    })
}

fn map_detail_player(
    document: Option<&Document>,
    report_schema: Option<i64>,
) -> BattleReportPlayer {
    let Some(document) = document else {
        return BattleReportPlayer::default();
    };
    let (avatar_url, frame_url) = resolve_avatar_override(document, report_schema).map_or_else(
        || (nested_string(document, &["avatar_url"]), nested_string(document, &["frame_url"])),
        |avatar_url| (Some(avatar_url), None),
    );

    BattleReportPlayer {
        player_id: nested_i64_exact(document, &["player_id"]).unwrap_or_default(),
        player_name: nested_string(document, &["player_name"]).unwrap_or_default(),
        alliance: BattleReportAlliance {
            abbreviation: nested_string(document, &["alliance", "abbreviation"])
                .unwrap_or_default(),
        },
        avatar_url,
        frame_url,
        tracking_key: nested_string(document, &["tracking_key"]),
        rally: nested_bool(document, &["rally"]),
        alliance_building_id: nested_i64_exact(document, &["alliance_building_id"]),
        castle: BattleReportCastle {
            x: nested_i64(document, &["castle", "x"]).unwrap_or_default(),
            y: nested_i64(document, &["castle", "y"]).unwrap_or_default(),
        },
        app_uid: nested_i64_exact(document, &["app_uid"]),
        commanders: map_detail_commanders(document),
        support_skills: map_detail_support_skills(nested_document(document, &["support_skills"])),
        auxiliary_skills: map_detail_auxiliary_skills(document),
    }
}

fn map_detail_commanders(document: &Document) -> BattleReportCommanderSet {
    BattleReportCommanderSet {
        primary: map_detail_commander(nested_document(document, &["commanders", "primary"])),
        secondary: map_detail_commander(nested_document(document, &["commanders", "secondary"])),
    }
}

fn map_detail_commander(document: Option<&Document>) -> BattleReportCommander {
    let Some(document) = document else {
        return BattleReportCommander::default();
    };

    BattleReportCommander {
        id: nested_i64_exact(document, &["id"]),
        awakened: nested_bool(document, &["awakened"]),
        level: nested_i64_exact(document, &["level"]),
        formation: nested_i64_exact(document, &["formation"]),
        equipment: nested_string(document, &["equipment"]),
        relics: map_detail_relics(document),
        skills: map_detail_skills(document),
        armaments: map_detail_armaments(document),
    }
}

fn map_detail_relics(document: &Document) -> Vec<BattleReportRelic> {
    let Some(relics) = nested_array(document, &["relics"]) else {
        return Vec::new();
    };

    relics
        .iter()
        .filter_map(Bson::as_document)
        .filter_map(|relic| nested_i64_exact(relic, &["id"]).map(|id| BattleReportRelic { id }))
        .collect()
}

fn map_detail_skills(document: &Document) -> Vec<BattleReportCommanderSkill> {
    let Some(skills) = nested_array(document, &["skills"]) else {
        return Vec::new();
    };

    skills
        .iter()
        .filter_map(Bson::as_document)
        .map(|skill| BattleReportCommanderSkill {
            id: nested_i64_exact(skill, &["id"]).unwrap_or_default(),
            level: nested_i64_exact(skill, &["level"]).unwrap_or_default(),
        })
        .collect()
}

fn map_detail_armaments(document: &Document) -> Vec<BattleReportArmament> {
    let Some(armaments) = nested_array(document, &["armaments"]) else {
        return Vec::new();
    };

    armaments
        .iter()
        .filter_map(Bson::as_document)
        .map(|armament| BattleReportArmament {
            affix: nested_string(armament, &["affix"]),
            buffs: nested_string(armament, &["buffs"]),
        })
        .collect()
}

fn map_detail_support_skills(document: Option<&Document>) -> BattleReportSupportSkills {
    let Some(document) = document else {
        return BattleReportSupportSkills::default();
    };

    BattleReportSupportSkills {
        enable: nested_bool(document, &["enable"]),
        skills: map_detail_support_skill_entries(document),
    }
}

fn map_detail_support_skill_entries(document: &Document) -> Vec<BattleReportSupportSkill> {
    let Some(skills) = nested_array(document, &["skills"]) else {
        return Vec::new();
    };

    skills
        .iter()
        .filter_map(Bson::as_document)
        .map(|skill| BattleReportSupportSkill {
            hero_id: nested_i64_exact(skill, &["hero_id"]).unwrap_or_default(),
            skill_id: nested_i64_exact(skill, &["skill_id"]).unwrap_or_default(),
            skill_level: nested_i64_exact(skill, &["skill_level"]).unwrap_or_default(),
        })
        .collect()
}

fn map_detail_auxiliary_skills(document: &Document) -> Vec<BattleReportAuxiliarySkill> {
    let Some(skills) = nested_array(document, &["auxiliary_skills"]) else {
        return Vec::new();
    };

    skills
        .iter()
        .filter_map(Bson::as_document)
        .map(|skill| BattleReportAuxiliarySkill {
            hero_id: nested_i64_exact(skill, &["hero_id"]).unwrap_or_default(),
            level: nested_i64_exact(skill, &["level"]).unwrap_or_default(),
            skill_id: nested_i64_exact(skill, &["skill_id"]).unwrap_or_default(),
        })
        .collect()
}

fn map_detail_summary(document: Option<&Document>) -> BattleReportSummary {
    let Some(document) = document else {
        return BattleReportSummary::default();
    };

    BattleReportSummary {
        sender: map_detail_summary_entry(nested_document(document, &["sender"])),
        opponent: map_detail_summary_entry(nested_document(document, &["opponent"])),
    }
}

fn map_detail_summary_entry(document: Option<&Document>) -> BattleReportSummaryEntry {
    let Some(document) = document else {
        return BattleReportSummaryEntry::default();
    };

    BattleReportSummaryEntry {
        kill_points: nested_i64(document, &["kill_points"]),
        dead: nested_i64(document, &["dead"]),
        severely_wounded: nested_i64(document, &["severely_wounded"]),
        slightly_wounded: nested_i64(document, &["slightly_wounded"]),
        remaining: nested_i64(document, &["remaining"]),
        troop_units: nested_i64(document, &["troop_units"]),
    }
}

fn map_detail_opponents(
    document: &Document,
    report_schema: Option<i64>,
) -> Vec<BattleReportOpponent> {
    let Some(opponents) = nested_array(document, &["opponents"]) else {
        return Vec::new();
    };

    opponents
        .iter()
        .filter_map(Bson::as_document)
        .map(|opponent| map_detail_opponent(opponent, report_schema))
        .collect()
}

fn map_detail_opponent(document: &Document, report_schema: Option<i64>) -> BattleReportOpponent {
    let base = map_detail_player(Some(document), report_schema);
    let start_tick = nested_i64(document, &["start_tick"]).unwrap_or_default();

    BattleReportOpponent {
        player_id: base.player_id,
        player_name: base.player_name,
        alliance: base.alliance,
        avatar_url: base.avatar_url,
        frame_url: base.frame_url,
        tracking_key: base.tracking_key,
        rally: base.rally,
        alliance_building_id: base.alliance_building_id,
        castle: base.castle,
        app_uid: base.app_uid,
        commanders: base.commanders,
        support_skills: base.support_skills,
        auxiliary_skills: base.auxiliary_skills,
        start_tick,
        end_tick: nested_i64(document, &["end_tick"]).unwrap_or(start_tick),
        attack: BattleReportAttack {
            x: nested_i64(document, &["attack", "x"]).unwrap_or_default(),
            y: nested_i64(document, &["attack", "y"]).unwrap_or_default(),
        },
        npc: BattleReportNpc {
            r#type: nested_i64_exact(document, &["npc", "type"]),
            b_type: nested_i64_exact(document, &["npc", "b_type"]),
        },
        battle_results: map_detail_battle_results(nested_document(document, &["battle_results"])),
    }
}

fn map_detail_battle_results(document: Option<&Document>) -> BattleReportBattleResults {
    let Some(document) = document else {
        return BattleReportBattleResults::default();
    };

    BattleReportBattleResults {
        sender: map_detail_battle_result(nested_document(document, &["sender"])),
        opponent: map_detail_battle_result(nested_document(document, &["opponent"])),
    }
}

fn map_detail_battle_result(document: Option<&Document>) -> BattleReportBattleResult {
    let Some(document) = document else {
        return BattleReportBattleResult::default();
    };

    BattleReportBattleResult {
        reinforcements_join: nested_i64(document, &["reinforcements_join"]),
        reinforcements_leave: nested_i64(document, &["reinforcements_leave"]),
        kill_points: nested_i64(document, &["kill_points"]),
        acclaim: nested_i64(document, &["acclaim"]),
        severely_wounded: nested_i64(document, &["severely_wounded"]),
        slightly_wounded: nested_i64(document, &["slightly_wounded"]),
        remaining: nested_i64(document, &["remaining"]),
        dead: nested_i64(document, &["dead"]),
        heal: nested_i64(document, &["heal"]),
        troop_units: nested_i64(document, &["troop_units"]),
        troop_units_max: nested_i64(document, &["troop_units_max"]),
        power: nested_i64(document, &["power"]),
        attack_power: nested_i64(document, &["attack_power"]),
        skill_power: nested_i64(document, &["skill_power"]),
    }
}

fn map_detail_timeline(document: Option<&Document>) -> BattleReportTimeline {
    let Some(document) = document else {
        return BattleReportTimeline::default();
    };

    BattleReportTimeline {
        start_timestamp: nested_i64(document, &["start_timestamp"]).unwrap_or_default(),
        start_tick: nested_i64(document, &["start_tick"]).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use mongodb::bson::{Bson, doc};

    use super::{
        build_battle_detail_filter, build_battle_detail_projection, map_battle_detail_document,
    };

    #[test]
    fn builds_filter_from_report_id() {
        let filter = build_battle_detail_filter("mail-123");
        assert_eq!(filter.get_str("metadata.mail_id").ok(), Some("mail-123"));
    }

    #[test]
    fn includes_required_fields_in_projection() {
        let projection = build_battle_detail_projection();

        assert_eq!(projection.get_i32("metadata.mail_id").ok(), Some(1));
        assert_eq!(projection.get_i32("metadata.schema").ok(), Some(1));
        assert_eq!(projection.get_i32("sender.structure_id").ok(), Some(1));
        assert_eq!(projection.get_i32("sender.character_type").ok(), Some(1));
        assert_eq!(projection.get_i32("opponents.structure_id").ok(), Some(1));
        assert_eq!(projection.get_i32("opponents.is_outpost").ok(), Some(1));
        assert_eq!(projection.get_i32("sender.commanders.primary.id").ok(), Some(1));
        assert_eq!(projection.get_i32("sender.commanders.primary.awakened").ok(), Some(1));
        assert_eq!(projection.get_i32("sender.commanders.primary.relics.id").ok(), Some(1));
        assert_eq!(projection.get_i32("sender.commanders.primary.skills.id").ok(), Some(1));
        assert_eq!(projection.get_i32("sender.support_skills.enable").ok(), Some(1));
        assert_eq!(projection.get_i32("sender.support_skills.skills.skill_id").ok(), Some(1));
        assert_eq!(projection.get_i32("sender.auxiliary_skills.skill_id").ok(), Some(1));
        assert_eq!(projection.get_i32("opponents.auxiliary_skills.skill_id").ok(), Some(1));
        assert_eq!(
            projection.get_i32("opponents.battle_results.opponent.kill_points").ok(),
            Some(1)
        );
    }

    #[test]
    fn maps_detail_document_when_required_fields_are_present() {
        let document = doc! {
            "metadata": {
                "mail_id": "mail-1",
                "mail_time": 123_i64,
                "mail_role": "sender",
                "kvk": true,
            },
            "sender": {
                "player_id": 1_i64,
                "player_name": "Alpha",
                "alliance": {
                    "abbreviation": "AAA",
                },
                "tracking_key": "tracking",
                "rally": true,
                "alliance_building_id": Bson::Null,
                "castle": { "x": 10_i64, "y": 20_i64 },
                "app_uid": 777_i64,
                "support_skills": {
                    "enable": true,
                    "skills": [
                        { "hero_id": 10_i64, "skill_id": 101_i64, "skill_level": 5_i64 },
                    ],
                },
                "auxiliary_skills": [
                    { "hero_id": 20_i64, "level": 4_i64, "skill_id": 202_i64 },
                ],
                "commanders": {
                    "primary": {
                        "id": 10_i64,
                        "awakened": true,
                        "level": 60_i64,
                        "formation": 3_i64,
                        "equipment": "{1:100}",
                        "relics": [
                            { "id": 10001_i64, "level": 2_i64 },
                        ],
                        "skills": [
                            { "id": 101_i64, "level": 5_i64 },
                        ],
                        "armaments": [
                            { "affix": "1_2", "buffs": "100_3" },
                        ],
                    },
                    "secondary": {
                        "id": 11_i64,
                        "level": 50_i64,
                        "skills": [
                            { "id": 201_i64, "level": 4_i64 },
                        ],
                    },
                },
            },
            "summary": {
                "sender": {
                    "kill_points": 100_i64,
                    "dead": 10_i64,
                },
                "opponent": {
                    "kill_points": 90_i64,
                },
            },
            "timeline": {
                "start_timestamp": 1_000_i64,
                "start_tick": 50_i64,
            },
            "opponents": [
                {
                    "player_id": 2_i64,
                    "player_name": "Bravo",
                    "alliance": {
                        "abbreviation": "BBB",
                    },
                    "start_tick": 60_i64,
                    "attack": { "x": 1_i64, "y": 2_i64 },
                    "npc": { "type": 5_i64, "b_type": 8_i64 },
                    "support_skills": {
                        "enable": false,
                        "skills": [],
                    },
                    "auxiliary_skills": [
                        { "hero_id": 30_i64, "level": 3_i64, "skill_id": 303_i64 },
                    ],
                    "battle_results": {
                        "sender": {
                            "kill_points": 40_i64,
                        },
                        "opponent": {
                            "kill_points": 30_i64,
                        },
                    },
                },
            ],
        };

        let mapped = map_battle_detail_document(&document).expect("detail should map");
        assert_eq!(mapped.metadata.mail_id, "mail-1");
        assert_eq!(mapped.metadata.mail_time, 123);
        assert_eq!(mapped.sender.player_name, "Alpha");
        assert_eq!(mapped.sender.commanders.primary.id, Some(10));
        assert_eq!(mapped.sender.commanders.primary.awakened, Some(true));
        assert_eq!(mapped.sender.commanders.primary.relics.len(), 1);
        assert_eq!(mapped.sender.commanders.primary.relics[0].id, 10001);
        assert_eq!(mapped.sender.commanders.primary.skills.len(), 1);
        assert_eq!(mapped.sender.commanders.primary.skills[0].id, 101);
        assert_eq!(mapped.sender.commanders.primary.armaments.len(), 1);
        assert_eq!(mapped.sender.support_skills.enable, Some(true));
        assert_eq!(mapped.sender.support_skills.skills.len(), 1);
        assert_eq!(mapped.sender.support_skills.skills[0].skill_id, 101);
        assert_eq!(mapped.sender.auxiliary_skills[0].skill_id, 202);
        assert_eq!(mapped.summary.sender.kill_points, Some(100));
        assert_eq!(mapped.opponents.len(), 1);
        assert_eq!(mapped.opponents[0].npc.r#type, Some(5));
        assert_eq!(mapped.opponents[0].support_skills.enable, Some(false));
        assert_eq!(mapped.opponents[0].auxiliary_skills[0].skill_id, 303);
        assert_eq!(mapped.timeline.start_tick, 50);
    }

    #[test]
    fn structure_replaces_sender_avatar_and_removes_frame() {
        let document = doc! {
            "metadata": {
                "mail_id": "16210617176935008431",
                "mail_time": 123_i64,
                "schema": 10_002_i64,
            },
            "sender": {
                "avatar_url": "https://example.com/avatar.png",
                "frame_url": "https://example.com/frame.png",
                "structure_id": 109_i64,
            },
        };

        let mapped = map_battle_detail_document(&document).expect("detail should map");

        assert_eq!(
            (mapped.sender.avatar_url.as_deref(), mapped.sender.frame_url.as_deref()),
            (Some("https://cdn.rokbattles.com/game/sprites/img_iconGVGBuilding16.png"), None,)
        );
    }

    #[test]
    fn alliance_building_replaces_opponent_avatar_and_removes_frame() {
        let document = doc! {
            "metadata": {
                "mail_id": "53118177176782122317",
                "mail_time": 123_i64,
                "schema": 320_i64,
            },
            "sender": {},
            "opponents": [{
                "avatar_url": "https://example.com/avatar.png",
                "frame_url": "https://example.com/frame.png",
                "character_type": 7_i64,
                "alliance_building_id": 3_i64,
            }],
        };

        let mapped = map_battle_detail_document(&document).expect("detail should map");
        let opponent = mapped.opponents.first().expect("opponent should map");

        assert_eq!(
            (opponent.avatar_url.as_deref(), opponent.frame_url.as_deref()),
            (Some("https://cdn.rokbattles.com/game/sprites/img_iconAlliMainHall.png"), None,)
        );
    }

    #[test]
    fn ordinary_participant_keeps_avatar_and_frame() {
        let document = doc! {
            "metadata": { "mail_id": "mail-1", "mail_time": 123_i64 },
            "sender": {
                "avatar_url": "https://example.com/avatar.png",
                "frame_url": "https://example.com/frame.png",
                "character_type": 1_i64,
                "alliance_building_id": 1_i64,
            },
        };

        let mapped = map_battle_detail_document(&document).expect("detail should map");

        assert_eq!(
            (mapped.sender.avatar_url.as_deref(), mapped.sender.frame_url.as_deref()),
            (Some("https://example.com/avatar.png"), Some("https://example.com/frame.png"),)
        );
    }

    #[test]
    fn structure_without_sprite_keeps_avatar_and_frame() {
        let document = doc! {
            "metadata": { "mail_id": "mail-1", "mail_time": 123_i64 },
            "sender": {
                "avatar_url": "https://example.com/avatar.png",
                "frame_url": "https://example.com/frame.png",
                "structure_id": 125_i64,
            },
        };

        let mapped = map_battle_detail_document(&document).expect("detail should map");

        assert_eq!(
            (mapped.sender.avatar_url.as_deref(), mapped.sender.frame_url.as_deref()),
            (Some("https://example.com/avatar.png"), Some("https://example.com/frame.png"),)
        );
    }

    #[test]
    fn skips_detail_document_without_required_metadata_fields() {
        let missing_mail_id = doc! {
            "metadata": {
                "mail_time": 123_i64,
            },
        };

        assert!(map_battle_detail_document(&missing_mail_id).is_none());
    }
}
