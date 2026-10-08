use std::{cmp::Ordering, collections::BTreeMap};

use mongodb::bson::Document;

use super::{
    MATCH_DELTA_MILLIS,
    mapper::{extract_mail_time_millis, parse_string},
};

#[derive(Debug)]
pub(crate) struct MatchedArkMailSet {
    pub battle_results: Document,
    pub battle_results_mail_id: Option<String>,
    pub battle_results_time_millis: i64,
    pub individual_results: Option<Document>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct MatchKey {
    time_millis: i64,
    mail_id_key: String,
    sequence: u32,
}

#[derive(Debug)]
struct MatchCandidate {
    key: MatchKey,
    mail_id: Option<String>,
    mail_time_millis: i64,
    doc: Document,
}

type CandidatePool = BTreeMap<MatchKey, MatchCandidate>;

pub(crate) fn match_ark_mails(
    battle_results: Vec<Document>,
    individual_results: Vec<Document>,
    max_delta_millis: i64,
) -> Vec<MatchedArkMailSet> {
    let safe_max_delta_millis =
        if max_delta_millis > 0 { max_delta_millis } else { MATCH_DELTA_MILLIS };

    let mut individual_results_pool = build_pool(individual_results);
    let mut matches = Vec::new();

    for (index, document) in battle_results.into_iter().enumerate() {
        let sequence = u32::try_from(index).unwrap_or(u32::MAX);
        let Some(primary) = to_candidate(document, sequence) else {
            continue;
        };

        let individual_results = consume_best_candidate(
            &mut individual_results_pool,
            primary.mail_time_millis,
            safe_max_delta_millis,
        );

        matches.push(MatchedArkMailSet {
            battle_results: primary.doc,
            battle_results_mail_id: primary.mail_id,
            battle_results_time_millis: primary.mail_time_millis,
            individual_results,
        });
    }

    matches
}

fn build_pool(documents: Vec<Document>) -> CandidatePool {
    let mut pool = CandidatePool::new();

    for (index, document) in documents.into_iter().enumerate() {
        let sequence = u32::try_from(index).unwrap_or(u32::MAX);
        if let Some(candidate) = to_candidate(document, sequence) {
            pool.insert(candidate.key.clone(), candidate);
        }
    }

    pool
}

fn consume_best_candidate(
    pool: &mut CandidatePool,
    primary_time_millis: i64,
    max_delta_millis: i64,
) -> Option<Document> {
    let best_key = choose_best_candidate_key(pool, primary_time_millis, max_delta_millis)?;
    let candidate = pool.remove(&best_key)?;

    Some(candidate.doc)
}

fn choose_best_candidate_key(
    pool: &CandidatePool,
    primary_time_millis: i64,
    max_delta_millis: i64,
) -> Option<MatchKey> {
    if pool.is_empty() {
        return None;
    }

    let probe =
        MatchKey { time_millis: primary_time_millis, mail_id_key: String::new(), sequence: 0 };

    let previous_key = pool.range(..=probe.clone()).next_back().map(|(key, _)| key.clone());
    let next_key = pool.range(probe.clone()..).next().map(|(key, _)| key.clone());

    let mut candidate_keys = Vec::with_capacity(2);
    if let Some(previous) = previous_key {
        candidate_keys.push(previous);
    }
    if let Some(next) = next_key
        && candidate_keys.first() != Some(&next)
    {
        candidate_keys.push(next);
    }

    let mut best_key: Option<MatchKey> = None;

    let max_delta_millis = u64::try_from(max_delta_millis).unwrap_or(u64::MAX);

    for candidate_key in candidate_keys {
        let Some(candidate) = pool.get(&candidate_key) else {
            continue;
        };

        if candidate.mail_time_millis.abs_diff(primary_time_millis) > max_delta_millis {
            continue;
        }

        let is_better = match best_key.as_ref().and_then(|key| pool.get(key)) {
            Some(current_best) => {
                is_better_candidate(candidate, current_best, primary_time_millis) == Ordering::Less
            }
            None => true,
        };

        if is_better {
            best_key = Some(candidate_key);
        }
    }

    best_key
}

fn is_better_candidate(
    candidate: &MatchCandidate,
    current_best: &MatchCandidate,
    primary_time_millis: i64,
) -> Ordering {
    let candidate_delta = candidate.mail_time_millis.abs_diff(primary_time_millis);
    let current_delta = current_best.mail_time_millis.abs_diff(primary_time_millis);

    match candidate_delta.cmp(&current_delta) {
        Ordering::Equal => match current_best.mail_time_millis.cmp(&candidate.mail_time_millis) {
            Ordering::Equal => {
                let candidate_id = candidate.mail_id.as_deref().unwrap_or("");
                let current_id = current_best.mail_id.as_deref().unwrap_or("");
                candidate_id.cmp(current_id)
            }
            order => order,
        },
        order => order,
    }
}

fn to_candidate(document: Document, sequence: u32) -> Option<MatchCandidate> {
    let metadata = document.get_document("metadata").ok()?;
    let mail_time_millis = extract_mail_time_millis(&document)?;
    let mail_id = parse_string(metadata.get("mail_id"));
    let mail_id_key = mail_id.clone().unwrap_or_default();

    Some(MatchCandidate {
        key: MatchKey { time_millis: mail_time_millis, mail_id_key, sequence },
        mail_id,
        mail_time_millis,
        doc: document,
    })
}

#[cfg(test)]
mod tests {
    use mongodb::bson::doc;

    use super::*;

    fn individual_mail_id(matched: &MatchedArkMailSet) -> Option<&str> {
        matched.individual_results.as_ref()?.get_document("metadata").ok()?.get_str("mail_id").ok()
    }

    #[test]
    fn matches_exact_timestamp_candidates() {
        let matched = match_ark_mails(
            vec![doc! { "metadata": { "mail_id": "r1", "mail_time": 1_700_000_000_i64 } }],
            vec![doc! { "metadata": { "mail_id": "n1", "mail_time": 1_700_000_000_i64 } }],
            60_000,
        );

        assert_eq!(matched.len(), 1);
        assert_eq!(individual_mail_id(&matched[0]), Some("n1"));
    }

    #[test]
    fn consumes_each_individual_result_only_once() {
        let matched = match_ark_mails(
            vec![
                doc! { "metadata": { "mail_id": "r1", "mail_time": 1_700_000_000_i64 } },
                doc! { "metadata": { "mail_id": "r2", "mail_time": 1_700_000_001_i64 } },
            ],
            vec![doc! { "metadata": { "mail_id": "n1", "mail_time": 1_700_000_000_i64 } }],
            60_000,
        );

        assert_eq!(matched.len(), 2);
        assert_eq!(individual_mail_id(&matched[0]), Some("n1"));
        assert!(matched[1].individual_results.is_none());
    }

    #[test]
    fn matches_bson_dates_with_string_encoded_timestamps() {
        let timestamp = mongodb::bson::DateTime::from_millis(1_700_000_000_000);
        let matched = match_ark_mails(
            vec![doc! { "metadata": { "mail_id": "r1", "mail_time": timestamp } }],
            vec![doc! { "metadata": { "mail_id": "n1", "mail_time": "1700000000000000" } }],
            60_000,
        );

        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].battle_results_time_millis, timestamp.timestamp_millis());
        assert_eq!(individual_mail_id(&matched[0]), Some("n1"));
    }

    #[test]
    fn prefers_newer_candidate_when_delta_is_equal() {
        let base_micros = 1_700_000_000_000_000_i64;

        let matched = match_ark_mails(
            vec![doc! { "metadata": { "mail_id": "r1", "mail_time": base_micros } }],
            vec![
                doc! { "metadata": { "mail_id": "older", "mail_time": base_micros - 100_000 } },
                doc! { "metadata": { "mail_id": "newer", "mail_time": base_micros + 100_000 } },
            ],
            500,
        );

        assert_eq!(individual_mail_id(&matched[0]), Some("newer"));
    }

    #[test]
    fn prefers_lexicographically_smallest_mail_id_for_same_timestamp() {
        let base_micros = 1_700_000_000_000_000_i64;

        let matched = match_ark_mails(
            vec![doc! { "metadata": { "mail_id": "r1", "mail_time": base_micros } }],
            vec![
                doc! { "metadata": { "mail_id": "z-id", "mail_time": base_micros } },
                doc! { "metadata": { "mail_id": "a-id", "mail_time": base_micros } },
            ],
            500,
        );

        assert_eq!(individual_mail_id(&matched[0]), Some("a-id"));
    }

    #[test]
    fn ignores_candidates_outside_match_window() {
        let base_micros = 1_700_000_000_000_000_i64;

        let matched = match_ark_mails(
            vec![doc! { "metadata": { "mail_id": "r1", "mail_time": base_micros } }],
            vec![doc! { "metadata": { "mail_id": "far", "mail_time": base_micros + 10_000_000 } }],
            500,
        );

        assert!(matched[0].individual_results.is_none());
    }
}
