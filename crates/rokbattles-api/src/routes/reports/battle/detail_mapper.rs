use mongodb::bson::{Document, doc};

pub(super) fn build_report_detail_filter(report_id: &str) -> Document {
    doc! { "metadata.mail_id": report_id }
}

pub(super) fn report_detail_projection() -> Document {
    let mut projection = Document::new();
    projection.insert("_id", 0);

    for field in [
        "metadata.mail_id",
        "metadata.mail_time",
        "metadata.mail_role",
        "metadata.kvk",
        "sender.player_id",
        "sender.player_name",
        "sender.alliance.abbreviation",
        "sender.avatar_url",
        "sender.frame_url",
        "sender.tracking_key",
        "sender.rally",
        "sender.alliance_building_id",
        "sender.castle.x",
        "sender.castle.y",
        "sender.app_uid",
        "sender.commanders.primary.id",
        "sender.commanders.primary.level",
        "sender.commanders.primary.formation",
        "sender.commanders.primary.equipment",
        "sender.commanders.primary.armaments.affix",
        "sender.commanders.primary.armaments.buffs",
        "sender.commanders.secondary.id",
        "sender.commanders.secondary.level",
        "sender.commanders.secondary.equipment",
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
        "opponents.castle.x",
        "opponents.castle.y",
        "opponents.app_uid",
        "opponents.tracking_key",
        "opponents.commanders.primary.id",
        "opponents.commanders.primary.level",
        "opponents.commanders.primary.formation",
        "opponents.commanders.primary.equipment",
        "opponents.commanders.primary.armaments.affix",
        "opponents.commanders.primary.armaments.buffs",
        "opponents.commanders.secondary.id",
        "opponents.commanders.secondary.level",
        "opponents.commanders.secondary.equipment",
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
        "opponents.battle_results.sender.watchtower_max",
        "opponents.battle_results.sender.watchtower",
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
        "opponents.battle_results.opponent.watchtower_max",
        "opponents.battle_results.opponent.watchtower",
        "opponents.battle_results.opponent.power",
        "opponents.battle_results.opponent.attack_power",
        "opponents.battle_results.opponent.skill_power",
    ] {
        projection.insert(field, 1);
    }

    projection
}

#[cfg(test)]
mod tests {
    use super::{build_report_detail_filter, report_detail_projection};

    #[test]
    fn builds_filter_from_report_id() {
        let filter = build_report_detail_filter("mail-123");
        assert_eq!(filter.get_str("metadata.mail_id").ok(), Some("mail-123"));
    }

    #[test]
    fn includes_required_fields_in_projection() {
        let projection = report_detail_projection();

        assert_eq!(projection.get_i32("metadata.mail_id").ok(), Some(1));
        assert_eq!(
            projection.get_i32("sender.commanders.primary.id").ok(),
            Some(1)
        );
        assert_eq!(
            projection
                .get_i32("opponents.battle_results.opponent.kill_points")
                .ok(),
            Some(1)
        );
    }
}
