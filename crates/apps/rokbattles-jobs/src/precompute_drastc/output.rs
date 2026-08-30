use mongodb::bson::{DateTime, Document, doc};
use rokbattles_drastc::{CategoryScore, DrastcConfidence, DrastcScore};

use super::model::PairingKey;

pub(super) fn build_drastc_document(
    key: PairingKey,
    score: &DrastcScore,
    confidence: &DrastcConfidence,
    refreshed_at: DateTime,
) -> Document {
    doc! {
        "primary_commander_id": key.primary_commander_id,
        "secondary_commander_id": key.secondary_commander_id,
        "drastc": {
            "samples": u64_to_i64(score.samples),
            "breakdown": {
                "damage": category_document(score.breakdown.damage),
                "rage": category_document(score.breakdown.rage),
                "assist": category_document(score.breakdown.assist),
                "sustainability": category_document(score.breakdown.sustainability),
                "trade": category_document(score.breakdown.trade),
                "consistency": category_document(score.breakdown.consistency),
            },
            "overall": score.overall,
            "confidence": {
                "score": confidence.score,
                "unique_governors": u64_to_i64(confidence.unique_governors),
                "effective_governors": confidence.effective_governors,
            },
        },
        "refreshed_at": refreshed_at,
    }
}

fn category_document(score: CategoryScore) -> Document {
    doc! { "value": score.value, "p10": score.p10, "p90": score.p90, "score": score.score }
}

fn u64_to_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use rokbattles_drastc::{
        BattleRecord, DrastcConfidence, DrastcModel, DrastcReferenceRanges, ReferenceRange,
        SOC_RAGE_TABLE,
    };

    use super::*;

    #[test]
    fn document_contains_only_pair_identity_drastc_and_refresh_time() {
        let mut model = DrastcModel::new();
        model.set_rage_table(SOC_RAGE_TABLE);
        model.set_reference_ranges(DrastcReferenceRanges {
            damage: ReferenceRange::new(1, 0.0, 4.0),
            sustainability: ReferenceRange::new(1, -2.0, 2.0),
            trade: ReferenceRange::new(1, 0.0, 2.0),
            consistency: ReferenceRange::new(1, 0.0, 1.0),
        });
        model.set_theoretical(579, 575);
        model.push(BattleRecord {
            sample_count: 2,
            total_duration_seconds: 60.0,
            kill_points: 0.0,
            opponent_kill_points: 0.0,
            opponent_dead: 0.0,
            opponent_severely_wounded: 0.0,
            opponent_slightly_wounded: 0.0,
            sender_dead: 0.0,
            sender_severely_wounded: 0.0,
            sender_slightly_wounded: 0.0,
            sender_healing: 0.0,
            decisive_battles: 0,
            wins: 0,
            positive_trades: 0,
        });
        let score = model.evaluate().expect("score");
        let document = build_drastc_document(
            PairingKey { primary_commander_id: 579, secondary_commander_id: 575 },
            &score,
            &DrastcConfidence { score: 0.5, unique_governors: 3, effective_governors: 2.5 },
            DateTime::from_millis(0),
        );

        assert_eq!(document.len(), 4);
        assert!(document.get_document("drastc").is_ok());
        assert!(!document.contains_key("strategies"));
    }
}
