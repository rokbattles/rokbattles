use super::types::ReportRowWithCursor;
pub(crate) use crate::routes::reports::common::map_context::BattleMapContext;
use crate::{
    db::ReportsStore, error::ApiError, routes::reports::common::map_context::load_map_snapshots,
};

pub(super) async fn enrich_map_context(
    store: &ReportsStore,
    rows: &mut [ReportRowWithCursor],
) -> Result<(), ApiError> {
    let snapshots = load_map_snapshots(store, rows.iter().map(|row| row.map_context)).await?;
    for row in rows {
        (row.item.kvk_mapcode, row.item.kvk_banner) = row.map_context.resolve(&snapshots);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use mongodb::bson::{Document, doc};

    const BANNER_BASE: &str = "https://cdn.rokbattles.com/game/banners/";

    fn snapshot() -> Document {
        doc! {
            "Id": 16053, "Order": 13249, "Mode": 16,
            "OpenTime": 1000, "CloseTime": 2000,
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
        assert_eq!(row.map_context.resolve(&[info]).0, "#C13249");
        assert_eq!(row.item.time_start, 1_789_929_193);
    }

    #[test]
    fn list_projection_and_serialization_keep_lookup_fields_private() {
        use super::super::list_mapper::{build_battle_list_projection, map_battle_list_document};
        let projection = build_battle_list_projection();
        for field in [
            "metadata.server_id",
            "metadata.kvk",
            "metadata.mail_role",
            "sender.kingdom_id",
            "sender.session",
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
        assert_eq!(value["kvkMapcode"], "#C13249");
        assert_eq!(value["kvkBanner"], format!("{BANNER_BASE}s4heroic_anthem_cover.png"));
        assert_eq!(value.as_object().expect("object").len(), 12);
        for private in ["serverId", "mapContext", "Order", "OpenTime", "CloseTime", "Mode"] {
            assert!(value.get(private).is_none(), "leaked {private}");
        }
    }
}
