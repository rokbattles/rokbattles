//! Commander catalog helpers shared by precompute jobs.

use std::collections::BTreeSet;

use crate::error::JobsError;

const COMMANDERS_YAML: &str = include_str!("../../../../datasets/commanders.yaml");

pub(crate) fn combat_lab_commander_ids() -> Result<Vec<i64>, JobsError> {
    let mut ids = BTreeSet::new();
    let mut current_id = None;
    let mut current_is_supported = false;

    for line in COMMANDERS_YAML.lines() {
        if let Some(id) = parse_top_level_commander_id(line) {
            if current_is_supported && let Some(previous_id) = current_id {
                ids.insert(previous_id);
            }

            current_id = Some(id);
            current_is_supported = false;
            continue;
        }

        if matches!(line.trim(), "rarity: legendary" | "rarity: epic") {
            current_is_supported = true;
        }
    }

    if current_is_supported && let Some(previous_id) = current_id {
        ids.insert(previous_id);
    }

    if ids.is_empty() {
        return Err(JobsError::MissingCombatLabCommanders);
    }

    Ok(ids.into_iter().collect())
}

fn parse_top_level_commander_id(line: &str) -> Option<i64> {
    let rest = line.strip_prefix("  ")?;
    if rest.starts_with(' ') {
        return None;
    }

    let id = rest.strip_suffix(':')?;
    id.parse::<i64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combat_lab_commander_ids_reads_expected_dataset_values() {
        let ids = combat_lab_commander_ids().expect("Combat Lab IDs");

        assert!(ids.contains(&509));
        assert!(ids.contains(&6));
        assert!(ids.contains(&179));
        assert!(ids.contains(&187));
        assert!(ids.contains(&3));
        assert!(ids.contains(&12));
        assert!(ids.contains(&537));
        assert!(!ids.contains(&22));
        assert!(!ids.contains(&33));
    }
}
