use std::collections::BTreeMap;

use mongodb::bson::Document;
use rokbattles_bson::bson_to_i64_exact;

pub(super) fn map_commander_skins(document: Option<&Document>) -> Option<BTreeMap<String, i64>> {
    document?
        .iter()
        .map(|(key, value)| {
            bson_to_i64_exact(value).filter(|id| *id >= 0).map(|id| (key.clone(), id))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use mongodb::bson::{Document, doc};
    use serde_json::json;

    use super::map_commander_skins;
    use crate::routes::reports::battle::{
        detail_mapper::{build_battle_detail_projection, map_battle_detail_document},
        list_mapper::{build_battle_list_projection, map_battle_list_document},
    };

    #[test]
    fn projects_cosmetics_for_every_commander_in_both_responses() {
        for projection in [build_battle_list_projection(), build_battle_detail_projection()] {
            for side in ["sender", "opponents"] {
                for slot in ["primary", "secondary"] {
                    let path = format!("{side}.commanders.{slot}.skins");

                    assert_eq!(projection.get_i32(&path), Ok(1), "{path}");
                }
            }
        }
    }

    #[test]
    fn serializes_each_commanders_cosmetics_without_mixing_slots_or_sides() {
        let document = doc! {
            "metadata": {"mail_id": "skins", "mail_time": 100},
            "sender": {"commanders": {
                "primary": {"skins": {"12": 70006, "21": 77004, "22": 100204}},
                "secondary": {"skins": {"22": 100201}},
            }},
            "opponents": [{"player_id": 123, "commanders": {
                "primary": {"skins": {"22": 100202}},
                "secondary": {"skins": {"22": 100203, "99": 42}},
            }}],
        };

        let list = serde_json::to_value(map_battle_list_document(&document).unwrap().item).unwrap();
        let detail = serde_json::to_value(map_battle_detail_document(&document).unwrap()).unwrap();

        for (side, detail_player) in
            [("sender", &detail["sender"]), ("opponent", &detail["opponents"][0])]
        {
            for slot in ["primary", "secondary"] {
                let source = if side == "sender" {
                    document.get_document("sender").unwrap()
                } else {
                    document.get_array("opponents").unwrap()[0].as_document().unwrap()
                };

                let expected = serde_json::to_value(
                    source
                        .get_document("commanders")
                        .unwrap()
                        .get_document(slot)
                        .unwrap()
                        .get_document("skins")
                        .unwrap(),
                )
                .unwrap();

                assert_eq!(list[side][format!("{slot}CommanderSkins")], expected);
                assert_eq!(detail_player["commanders"][slot]["skins"], expected);
            }
        }
    }

    #[test]
    fn older_reports_serialize_missing_cosmetics_as_null() {
        let document = doc! {
            "metadata": {"mail_id": "old", "mail_time": 100},
            "sender": {"commanders": {"primary": {"skins": null}}},
            "opponents": [{"player_id": 123}],
        };

        let list = serde_json::to_value(map_battle_list_document(&document).unwrap().item).unwrap();
        let detail = serde_json::to_value(map_battle_detail_document(&document).unwrap()).unwrap();

        for (side, detail_player) in
            [("sender", &detail["sender"]), ("opponent", &detail["opponents"][0])]
        {
            for slot in ["primary", "secondary"] {
                assert_eq!(list[side][format!("{slot}CommanderSkins")], json!(null));
                assert_eq!(detail_player["commanders"][slot]["skins"], json!(null));
            }
        }
    }

    #[test]
    fn preserves_empty_skin_maps_and_rejects_invalid_ids() {
        assert_eq!(map_commander_skins(Some(&Document::new())), Some(Default::default()));

        for document in [doc! {"22": -1}, doc! {"22": "100204"}, doc! {"22": 1.5}] {
            assert_eq!(map_commander_skins(Some(&document)), None);
        }
    }
}
