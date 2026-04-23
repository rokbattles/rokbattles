use std::path::Path;

use mail_processor_sdk::ProcessError;
use serde::Serialize;
use serde_json::Value;

use super::{
    classify::{
        MAIL_TYPE_ALLIANCE_AOO_BATTLE_INFO, MAIL_TYPE_ALLIANCE_AOO_BATTLE_RESULTS,
        MAIL_TYPE_ALLIANCE_AOO_INDIVIDUAL_RESULTS, MAIL_TYPE_SYSTEM_BARBARIAN_FORT,
        classify_processable_mail_type,
    },
    paths::processed_output_path,
};
use crate::{MailCliError, fs_utils::write_json_file};

pub(super) fn write_processed_json(
    output_dir: &Path,
    input_path: &Path,
    value: &Value,
    pretty: bool,
) -> Result<(), MailCliError> {
    let processed_input = match value {
        Value::Object(_) => Some(value),
        Value::Array(items) => match items.as_slice() {
            [item] if item.is_object() => Some(item),
            _ => None,
        },
        _ => None,
    };
    let Some(processed_input) = processed_input else {
        return Ok(());
    };

    match classify_processable_mail_type(processed_input) {
        Some("BarCanyonKillBoss") => process_and_write_output(
            output_dir,
            input_path,
            processed_input,
            pretty,
            mail_processor_barcanyonkillboss::process_parallel,
        ),
        Some("Rss") => process_and_write_output(
            output_dir,
            input_path,
            processed_input,
            pretty,
            mail_processor_rss::process_parallel,
        ),
        Some("Battle") => process_and_write_output(
            output_dir,
            input_path,
            processed_input,
            pretty,
            mail_processor_battle::process_parallel,
        ),
        Some("DuelBattle2") => process_and_write_output(
            output_dir,
            input_path,
            processed_input,
            pretty,
            mail_processor_duelbattle2::process_parallel,
        ),
        Some(MAIL_TYPE_SYSTEM_BARBARIAN_FORT) => process_and_write_output(
            output_dir,
            input_path,
            processed_input,
            pretty,
            mail_processor_system_barbarianfort::process_parallel,
        ),
        Some(MAIL_TYPE_ALLIANCE_AOO_BATTLE_RESULTS) => process_and_write_output(
            output_dir,
            input_path,
            processed_input,
            pretty,
            mail_processor_alliance_aoobattleresults::process_parallel,
        ),
        Some(MAIL_TYPE_ALLIANCE_AOO_BATTLE_INFO) => process_and_write_output(
            output_dir,
            input_path,
            processed_input,
            pretty,
            mail_processor_alliance_aoobattleinfo::process_parallel,
        ),
        Some(MAIL_TYPE_ALLIANCE_AOO_INDIVIDUAL_RESULTS) => process_and_write_output(
            output_dir,
            input_path,
            processed_input,
            pretty,
            mail_processor_alliance_aooindividualresults::process_parallel,
        ),
        _ => Ok(()),
    }
}

fn process_and_write_output<T, F>(
    output_dir: &Path,
    input_path: &Path,
    processed_input: &Value,
    pretty: bool,
    processor: F,
) -> Result<(), MailCliError>
where
    T: Serialize,
    F: FnOnce(&Value) -> Result<T, ProcessError>,
{
    let processed = processor(processed_input)
        .map_err(|source| MailCliError::Process { source, path: input_path.to_path_buf() })?;
    let output_path = processed_output_path(output_dir, input_path)?;
    write_json_file(&output_path, &processed, pretty)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use serde_json::{Value, json};

    use super::*;
    use crate::run::paths::processed_output_path;

    #[test]
    fn write_processed_json_skips_unknown_type() {
        let temp = tempfile::tempdir().expect("temp dir");
        let input = temp.path().join("sample.mail");
        let value = json!({ "type": "Unknown" });

        write_processed_json(temp.path(), &input, &value, true).unwrap();
        let output = processed_output_path(temp.path(), &input).unwrap();
        assert!(!output.exists());
    }

    #[test]
    fn write_processed_json_skips_multi_item_arrays() {
        let temp = tempfile::tempdir().expect("temp dir");
        let input = temp.path().join("sample.mail");
        let value = json!([{ "type": "Battle" }, { "type": "Battle" }]);

        write_processed_json(temp.path(), &input, &value, true).unwrap();
        let output = processed_output_path(temp.path(), &input).unwrap();
        assert!(!output.exists());
    }

    #[test]
    fn write_processed_json_skips_scalar_roots() {
        let temp = tempfile::tempdir().expect("temp dir");
        let input = temp.path().join("sample.mail");
        let value = json!("Battle");

        write_processed_json(temp.path(), &input, &value, true).unwrap();
        let output = processed_output_path(temp.path(), &input).unwrap();
        assert!(!output.exists());
    }

    #[test]
    fn write_processed_json_writes_battle_metadata() {
        let temp = tempfile::tempdir().expect("temp dir");
        let input = temp.path().join("sample.mail");
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Battle/Persistent.Mail.1002579517552941234.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        write_processed_json(temp.path(), &input, &value, true).unwrap();
        let output = processed_output_path(temp.path(), &input).unwrap();
        let output_json = fs::read_to_string(output).expect("read processed");
        let parsed: Value = serde_json::from_str(&output_json).expect("parse processed");
        assert_eq!(parsed["metadata"]["mail_id"], json!("1002579517552941234"));
    }

    #[test]
    fn write_processed_json_defaults_missing_battle_castle_level() {
        let temp = tempfile::tempdir().expect("temp dir");
        let input = temp.path().join("sample.mail");
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Battle/Persistent.Mail.30346123155694137715.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        write_processed_json(temp.path(), &input, &value, true).unwrap();
        let output = processed_output_path(temp.path(), &input).unwrap();
        let output_json = fs::read_to_string(output).expect("read processed");
        let parsed: Value = serde_json::from_str(&output_json).expect("parse processed");
        assert_eq!(parsed["sender"]["castle"]["level"], json!(0));
    }

    #[test]
    fn write_processed_json_preserves_negative_hurt_in_battle_sample() {
        let temp = tempfile::tempdir().expect("temp dir");
        let input = temp.path().join("sample.mail");
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Battle/Persistent.Mail.47051167177023603224.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        write_processed_json(temp.path(), &input, &value, true).unwrap();
        let output = processed_output_path(temp.path(), &input).unwrap();
        let output_json = fs::read_to_string(output).expect("read processed");
        let parsed: Value = serde_json::from_str(&output_json).expect("parse processed");

        let opponents = parsed["opponents"].as_array().expect("opponents array");
        let entry = opponents
            .iter()
            .find(|opponent| opponent["attack"]["id"] == json!("560025001"))
            .expect("edge-case attack entry");
        assert_eq!(entry["battle_results"]["sender"]["slightly_wounded"], json!(-565));
    }

    #[test]
    fn write_processed_json_handles_singleton_array() {
        let temp = tempfile::tempdir().expect("temp dir");
        let input = temp.path().join("sample.mail");
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Battle/Persistent.Mail.33830971176980291131.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        assert!(matches!(value, Value::Array(_)));

        write_processed_json(temp.path(), &input, &value, true).unwrap();
        let output = processed_output_path(temp.path(), &input).unwrap();
        let output_json = fs::read_to_string(output).expect("read processed");
        let parsed: Value = serde_json::from_str(&output_json).expect("parse processed");
        assert_eq!(parsed["metadata"]["mail_id"], json!("33830971176980291131"));
    }

    #[test]
    fn write_processed_json_writes_barcanyonkillboss_metadata_and_npc() {
        let temp = tempfile::tempdir().expect("temp dir");
        let input = temp.path().join("sample.mail");
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/BarCanyonKillBoss/Persistent.Mail.21162669176948646831.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        write_processed_json(temp.path(), &input, &value, true).unwrap();
        let output = processed_output_path(temp.path(), &input).unwrap();
        let output_json = fs::read_to_string(output).expect("read processed");
        let parsed: Value = serde_json::from_str(&output_json).expect("parse processed");
        assert_eq!(parsed["metadata"]["mail_id"], json!("21162669176948646831"));
        assert_eq!(parsed["npc"]["type"], json!(102000055));
        assert_eq!(parsed["npc"]["level"], json!(25));
    }

    #[test]
    fn write_processed_json_writes_duelbattle2_metadata() {
        let temp = tempfile::tempdir().expect("temp dir");
        let input = temp.path().join("sample.mail");
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/DuelBattle2/Persistent.Mail.4197312176618249531.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        write_processed_json(temp.path(), &input, &value, true).unwrap();
        let output = processed_output_path(temp.path(), &input).unwrap();
        let output_json = fs::read_to_string(output).expect("read processed");
        let parsed: Value = serde_json::from_str(&output_json).expect("parse processed");
        assert_eq!(parsed["metadata"]["mail_id"], json!("4197312176618249531"));
    }

    #[test]
    fn write_processed_json_writes_system_barbarianfort_fields() {
        let temp = tempfile::tempdir().expect("temp dir");
        let input = temp.path().join("sample.mail");
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/System/Persistent.Mail.87938122177133895831.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        write_processed_json(temp.path(), &input, &value, true).unwrap();
        let output = processed_output_path(temp.path(), &input).unwrap();
        let output_json = fs::read_to_string(output).expect("read processed");
        let parsed: Value = serde_json::from_str(&output_json).expect("parse processed");
        assert_eq!(parsed["metadata"]["mail_id"], json!("87938122177133895831"));
        assert_eq!(parsed["body"]["target_name"], json!("Level9"));
        assert_eq!(parsed["body"]["sub_type"], json!(11));
        assert_eq!(parsed["body"]["sub_param"], json!(1));
        assert_eq!(parsed["rewards"].as_array().unwrap().len(), 4);
    }

    #[test]
    fn write_processed_json_writes_rss_fields() {
        let temp = tempfile::tempdir().expect("temp dir");
        let input = temp.path().join("sample.mail");
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Rss/Persistent.Mail.113164877177212776431.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        write_processed_json(temp.path(), &input, &value, true).unwrap();
        let output = processed_output_path(temp.path(), &input).unwrap();
        let output_json = fs::read_to_string(output).expect("read processed");
        let parsed: Value = serde_json::from_str(&output_json).expect("parse processed");
        assert_eq!(parsed["metadata"]["mail_id"], json!("113164877177212776431"));
        assert_eq!(parsed["rss"]["rss_type"], json!(1));
        assert_eq!(parsed["rss"]["rss_value"], json!(4104.32));
        assert_eq!(parsed["rss"]["rss_bonus"], json!(232));
    }

    #[test]
    fn write_processed_json_writes_alliance_aoo_battle_results_metadata() {
        let temp = tempfile::tempdir().expect("temp dir");
        let input = temp.path().join("sample.mail");
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.102185423177177256731.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        write_processed_json(temp.path(), &input, &value, true).unwrap();
        let output = processed_output_path(temp.path(), &input).unwrap();
        let output_json = fs::read_to_string(output).expect("read processed");
        let parsed: Value = serde_json::from_str(&output_json).expect("parse processed");
        assert_eq!(parsed["metadata"]["mail_id"], json!("102185423177177256731"));
        assert_eq!(parsed["metadata"]["mail_receiver"], json!("player_71738515"));
        assert_eq!(parsed["metadata"]["custom"], json!(false));
    }

    #[test]
    fn write_processed_json_writes_alliance_aoo_battle_info_metadata() {
        let temp = tempfile::tempdir().expect("temp dir");
        let input = temp.path().join("sample.mail");
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.102185425177177256731.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        write_processed_json(temp.path(), &input, &value, true).unwrap();
        let output = processed_output_path(temp.path(), &input).unwrap();
        let output_json = fs::read_to_string(output).expect("read processed");
        let parsed: Value = serde_json::from_str(&output_json).expect("parse processed");
        assert_eq!(parsed["metadata"]["mail_id"], json!("102185425177177256731"));
        assert_eq!(parsed["metadata"]["mail_receiver"], json!("player_71738515"));
    }

    #[test]
    fn write_processed_json_writes_alliance_custom_battle_results_metadata() {
        let temp = tempfile::tempdir().expect("temp dir");
        let input = temp.path().join("sample.mail");
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.6906962177237730831.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        write_processed_json(temp.path(), &input, &value, true).unwrap();
        let output = processed_output_path(temp.path(), &input).unwrap();
        let output_json = fs::read_to_string(output).expect("read processed");
        let parsed: Value = serde_json::from_str(&output_json).expect("parse processed");
        assert_eq!(parsed["metadata"]["mail_id"], json!("6906962177237730831"));
        assert_eq!(parsed["metadata"]["mail_receiver"], json!("player_71738515"));
        assert_eq!(parsed["metadata"]["custom"], json!(true));
        assert_eq!(parsed["alliances"][0]["alliance"]["id"], json!(1));
        assert!(parsed["alliances"][0]["alliance"]["abbreviation"].is_null());
    }

    #[test]
    fn write_processed_json_writes_alliance_aoo_individual_results_metadata() {
        let temp = tempfile::tempdir().expect("temp dir");
        let input = temp.path().join("sample.mail");
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.102185429177177256731.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        write_processed_json(temp.path(), &input, &value, true).unwrap();
        let output = processed_output_path(temp.path(), &input).unwrap();
        let output_json = fs::read_to_string(output).expect("read processed");
        let parsed: Value = serde_json::from_str(&output_json).expect("parse processed");
        assert_eq!(parsed["metadata"]["mail_id"], json!("102185429177177256731"));
        assert_eq!(parsed["metadata"]["mail_receiver"], json!("player_71738515"));
        assert_eq!(parsed["metadata"]["custom"], json!(false));
    }

    #[test]
    fn write_processed_json_writes_alliance_custom_individual_results_metadata() {
        let temp = tempfile::tempdir().expect("temp dir");
        let input = temp.path().join("sample.mail");
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.6906964177237730831.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        write_processed_json(temp.path(), &input, &value, true).unwrap();
        let output = processed_output_path(temp.path(), &input).unwrap();
        let output_json = fs::read_to_string(output).expect("read processed");
        let parsed: Value = serde_json::from_str(&output_json).expect("parse processed");
        assert_eq!(parsed["metadata"]["mail_id"], json!("6906964177237730831"));
        assert_eq!(parsed["metadata"]["mail_receiver"], json!("player_71738515"));
        assert_eq!(parsed["metadata"]["custom"], json!(true));
        assert_eq!(parsed["body"]["win"], json!(false));
        assert!(parsed["body"]["team"].is_null());
    }
}
