#![forbid(unsafe_code)]

//! Parses ScoutReport mail reports.

mod character;
mod content;
mod durability;
mod metadata;
mod resources;
mod tower;
mod units;

pub use mail_processor_sdk::{ExtractError, Section};
use mail_processor_sdk::{ProcessError, ProcessedMail, Processor};
use serde_json::Value;

/// Runs the ScoutReport parser.
pub fn process(input: &Value) -> Result<ProcessedMail, ProcessError> {
    processor().process(input)
}

fn processor() -> Processor {
    Processor::new(vec![
        Box::new(metadata::MetadataExtractor::new()),
        Box::new(character::CharacterExtractor::new()),
        Box::new(resources::ResourcesExtractor::new()),
        Box::new(durability::DurabilityExtractor::new()),
        Box::new(tower::TowerExtractor::new()),
        Box::new(units::UnitsExtractor::new()),
    ])
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use serde_json::{Value, json};

    use super::*;

    #[test]
    fn process_extracts_metadata_character_and_resources_from_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/ScoutReport/Persistent.Mail.136953280177843782931.json");
        let raw = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&raw).expect("parse sample");

        let processed = process(&value).expect("process sample");
        let processed_json = serde_json::to_value(processed).expect("serialize processed");

        assert_eq!(processed_json["metadata"]["mail_id"], json!("136953280177843782931"));
        assert_eq!(processed_json["metadata"]["mail_receiver"], json!("player_71738515"));
        assert_eq!(processed_json["metadata"]["server_id"], json!(1804));
        assert_eq!(processed_json["metadata"]["mail_time"], json!(1778437829182528u64));
        assert_eq!(processed_json["character"]["alliance_building_type"], json!(0));
        assert_eq!(
            processed_json["character"]["alliance"],
            json!({
                "abbreviation": "SO4L",
                "id": 4805441,
                "logo": "4_3_15_1_0_3",
                "name": "魂社会SoulReapers",
            })
        );
        assert_eq!(processed_json["character"]["scout_type"], json!(3));
        assert_eq!(
            processed_json["character"]["position"],
            json!({ "x": 3829.63671875, "y": 3963.79296875 })
        );
        assert_eq!(
            processed_json["character"]["avatar_url"],
            json!("http://imimg.lilithcdn.com/roc/img_player_head06.png")
        );
        assert_eq!(
            processed_json["character"]["frame_url"],
            json!("http://imimg.lilithcdn.com/roc/img_ProfileBg220x220_a.png")
        );
        assert_eq!(processed_json["character"]["player_id"], json!(37556801));
        assert_eq!(processed_json["character"]["player_name"], json!("Benbu"));
        assert_eq!(
            processed_json["resources"],
            json!([
                { "type": 1, "value": 97898017 },
                { "type": 2, "value": 97678576 },
                { "type": 3, "value": 96266730 },
                { "type": 4, "value": 48852948 },
            ])
        );
        assert_eq!(
            processed_json["durability"],
            json!({
                "interval": 2000,
                "max": 40000,
                "next_update_time": 9223372036854775808u64,
                "num": 40000,
                "speed": 0,
                "state": 1,
                "state_end_time": 0,
            })
        );
        assert_eq!(
            processed_json["tower"],
            json!({
                "hp": 50000,
                "hp_speed": 500,
                "interval": 60000,
                "level": 25,
                "next_update_time": 0,
            })
        );
        assert_eq!(
            processed_json["units"][0],
            json!({ "id": 4, "count": 68704, "max_count": 173294 })
        );
        assert_eq!(
            processed_json["units"][6],
            json!({ "id": 16, "count": 666526, "max_count": 666526 })
        );
        assert_eq!(processed_json.as_object().map(serde_json::Map::len), Some(6));
    }
}
