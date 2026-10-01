//! Mutable mail fields used to decide whether an existing raw mail should be replaced.

use rokbattles_mail_registry::normalize_mail_root;
use serde_json::{Map, Value};

use crate::error::ApiError;

const MUTABLE_FIELDS: [&str; 8] = [
    "unread",
    "box",
    "prevBox",
    "reportMerge",
    "holdTag",
    "addition",
    "lastViewTs",
    "needPopupTips",
];

pub(crate) fn may_update_mutable_metadata(
    existing: &Value,
    incoming: &Value,
) -> Result<bool, ApiError> {
    let existing = normalize_mail_root(existing)
        .ok_or_else(|| ApiError::internal("stored mail has an invalid root"))?;
    let incoming = normalize_mail_root(incoming)
        .ok_or_else(|| ApiError::bad_request("incoming mail has an invalid root"))?;

    // A mutable flag cannot authorize replacement of any other content. Compare
    // the complete decoded objects, including fields unknown to this service;
    // the codec preserves them, and normalize_mail_root only checks root shape.
    if !immutable_content_matches(existing, incoming) {
        return Ok(false);
    }

    Ok(MUTABLE_FIELDS.iter().any(|field| existing.get(*field) != incoming.get(*field))
        || attachment_statuses(existing) != attachment_statuses(incoming))
}

fn immutable_content_matches(existing: &Value, incoming: &Value) -> bool {
    let (Some(existing), Some(incoming)) = (existing.as_object(), incoming.as_object()) else {
        return false;
    };
    fields_match(existing, incoming, &MUTABLE_FIELDS, |key, left, right| {
        if key == "attachments" {
            match (left.as_array(), right.as_array()) {
                (Some(left), Some(right)) => {
                    left.len() == right.len()
                        && left.iter().zip(right).all(|(left, right)| {
                            match (left.as_object(), right.as_object()) {
                                (Some(left), Some(right)) => {
                                    fields_match(left, right, &["status"], |_key, left, right| {
                                        left == right
                                    })
                                }
                                _ => left == right,
                            }
                        })
                }
                _ => left == right,
            }
        } else {
            left == right
        }
    })
}

fn fields_match(
    left: &Map<String, Value>,
    right: &Map<String, Value>,
    excluded: &[&str],
    equal: impl Fn(&str, &Value, &Value) -> bool,
) -> bool {
    let retained = |key: &str| !excluded.contains(&key);
    left.keys().filter(|key| retained(key)).count()
        == right.keys().filter(|key| retained(key)).count()
        && left
            .iter()
            .filter(|(key, _value)| retained(key))
            .all(|(key, value)| right.get(key).is_some_and(|other| equal(key, value, other)))
}

fn attachment_statuses(mail: &Value) -> Vec<Option<&Value>> {
    mail.get("attachments")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|attachment| attachment.get("status"))
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    fn mail() -> Value {
        json!({
            "id": "mail-1",
            "receiver": "player_123",
            "sender": "player_456",
            "type": "Battle",
            "time": 1234,
            "opaqueMetadata": {"futureField": [0, 255, 1]},
            "unread": true,
            "attachments": [{ "id": 1, "loot": [{ "type": 2 }], "status": 0 }],
            "box": "Report",
            "prevBox": "",
            "reportMerge": 0,
            "holdTag": 0,
            "addition": "",
            "lastViewTs": 0,
            "needPopupTips": false,
            "body": {
                "content": {
                    "Attacks": {
                        "attack-1": {
                            "Damage": { "Death": 10 },
                            "attack_idt_key": "attack-1"
                        }
                    }
                }
            }
        })
    }

    #[test]
    fn detects_each_allowlisted_top_level_field_change() {
        let existing = mail();
        let cases = [
            ("unread", json!(false)),
            ("box", json!("Archive")),
            ("prevBox", json!("Report")),
            ("reportMerge", json!(1)),
            ("holdTag", json!(1)),
            ("addition", json!("updated")),
            ("lastViewTs", json!(1234)),
            ("needPopupTips", json!(true)),
        ];

        for (field, value) in cases {
            let mut incoming = mail();
            incoming[field] = value;

            assert!(
                may_update_mutable_metadata(&existing, &incoming).expect("compare metadata"),
                "{field} should be treated as mutable"
            );
        }
    }

    #[test]
    fn detects_attachment_status_change() {
        let existing = mail();
        let mut incoming = mail();
        incoming["attachments"][0]["status"] = json!(1);

        assert!(may_update_mutable_metadata(&existing, &incoming).expect("compare metadata"));
    }

    #[test]
    fn ignores_non_status_attachment_changes() {
        let existing = mail();
        let mut incoming = mail();
        incoming["attachments"][0]["loot"][0]["type"] = json!(9);

        assert!(!may_update_mutable_metadata(&existing, &incoming).expect("compare metadata"));
    }

    #[test]
    fn ignores_substantive_battle_changes() {
        let existing = mail();
        let mut incoming = mail();
        incoming["body"]["content"]["Attacks"]["attack-1"]["Damage"]["Death"] = json!(99);

        assert!(!may_update_mutable_metadata(&existing, &incoming).expect("compare metadata"));
    }

    #[test]
    fn ignores_viewer_derived_battle_changes() {
        let existing = mail();
        let mut incoming = mail();
        incoming["body"]["content"]["Attacks"]["attack-1"]["attack_idt_key"] = json!("derived-key");

        assert!(!may_update_mutable_metadata(&existing, &incoming).expect("compare metadata"));
    }

    #[test]
    fn mutable_change_cannot_authorize_body_receiver_or_unknown_field_replacement() {
        let existing = mail();
        let mut cases = Vec::new();
        let mut body = mail();
        body["body"]["content"]["Attacks"]["attack-1"]["Damage"]["Death"] = json!(99);
        cases.push(body);
        let mut receiver = mail();
        receiver["receiver"] = json!("another-player");
        cases.push(receiver);
        let mut opaque = mail();
        opaque["opaqueMetadata"]["futureField"] = json!([1, 2, 3]);
        cases.push(opaque);
        let mut added = mail();
        added["futureUnknownField"] = json!({"opaque": [1, 2, 3]});
        cases.push(added);
        let mut removed = mail();
        removed.as_object_mut().expect("object").remove("id");
        cases.push(removed);

        for mut incoming in cases {
            incoming["unread"] = json!(false);
            assert!(!may_update_mutable_metadata(&existing, &incoming).expect("compare"));
        }
    }

    #[test]
    fn attachment_status_cannot_authorize_content_identity_or_order_changes() {
        let mut existing = mail();
        existing["attachments"].as_array_mut().expect("attachments").push(json!({
            "id": 2, "loot": [{"type": 3}], "status": 0
        }));
        let mut changed_content = existing.clone();
        changed_content["attachments"][0]["loot"][0]["type"] = json!(9);
        let mut changed_identity = existing.clone();
        changed_identity["attachments"][0]["id"] = json!(9);
        let mut changed_order = existing.clone();
        changed_order["attachments"].as_array_mut().expect("attachments").swap(0, 1);
        let mut changed_count = existing.clone();
        changed_count["attachments"].as_array_mut().expect("attachments").pop();

        for mut incoming in [changed_content, changed_identity, changed_order, changed_count] {
            incoming["attachments"][0]["status"] = json!(1);
            assert!(!may_update_mutable_metadata(&existing, &incoming).expect("compare"));
        }
    }

    #[test]
    fn approved_mutable_fields_may_be_added_or_removed_without_changing_content() {
        let existing = mail();
        let mut incoming = mail();
        incoming.as_object_mut().expect("object").remove("unread");
        assert!(may_update_mutable_metadata(&existing, &incoming).expect("removed"));
        assert!(may_update_mutable_metadata(&incoming, &existing).expect("added"));
    }
}
