//! Mutable mail fields used to decide whether an existing raw mail should be replaced.

use rokbattles_mail_registry::normalize_mail_root;
use serde_json::Value;

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

pub(crate) fn mutable_metadata_differs(
    existing: &Value,
    incoming: &Value,
) -> Result<bool, ApiError> {
    let existing = normalize_mail_root(existing)
        .ok_or_else(|| ApiError::internal("stored mail has an invalid root"))?;
    let incoming = normalize_mail_root(incoming)
        .ok_or_else(|| ApiError::bad_request("incoming mail has an invalid root"))?;

    Ok(MUTABLE_FIELDS.iter().any(|field| existing.get(*field) != incoming.get(*field))
        || attachment_statuses(existing) != attachment_statuses(incoming))
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
                mutable_metadata_differs(&existing, &incoming).expect("compare metadata"),
                "{field} should be treated as mutable"
            );
        }
    }

    #[test]
    fn detects_attachment_status_change() {
        let existing = mail();
        let mut incoming = mail();
        incoming["attachments"][0]["status"] = json!(1);

        assert!(mutable_metadata_differs(&existing, &incoming).expect("compare metadata"));
    }

    #[test]
    fn ignores_non_status_attachment_changes() {
        let existing = mail();
        let mut incoming = mail();
        incoming["attachments"][0]["loot"][0]["type"] = json!(9);

        assert!(!mutable_metadata_differs(&existing, &incoming).expect("compare metadata"));
    }

    #[test]
    fn ignores_substantive_battle_changes() {
        let existing = mail();
        let mut incoming = mail();
        incoming["body"]["content"]["Attacks"]["attack-1"]["Damage"]["Death"] = json!(99);

        assert!(!mutable_metadata_differs(&existing, &incoming).expect("compare metadata"));
    }

    #[test]
    fn ignores_viewer_derived_battle_changes() {
        let existing = mail();
        let mut incoming = mail();
        incoming["body"]["content"]["Attacks"]["attack-1"]["attack_idt_key"] = json!("derived-key");

        assert!(!mutable_metadata_differs(&existing, &incoming).expect("compare metadata"));
    }
}
