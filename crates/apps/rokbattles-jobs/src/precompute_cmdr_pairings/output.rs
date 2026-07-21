use mongodb::bson::{Bson, DateTime, Document, doc};
use rokbattles_drastc::{DrastcConfidence, DrastcScore};

use super::model::{
    PairingKey, PairingRawTotals, PairingStrategies, PairingSummary, Strategy, StrategyRawTotals,
};

fn finalize_summary(raw: PairingRawTotals) -> PairingSummary {
    PairingSummary {
        total_battles: raw.total_battles,
        kill_points_gained: raw.kill_points_gained,
        kill_points_lost: raw.kill_points_lost,
        avg_trade_percentage: divide(raw.trade_percentage_total, raw.total_battles as f64),
        weighted_trade_percentage: compute_trade_percentage(
            raw.kill_points_gained,
            raw.kill_points_lost,
        ),
        avg_battle_duration: divide(raw.battle_duration_total as f64, raw.total_battles as f64),
        total_battle_duration: raw.battle_duration_total,
        severely_wounded_inflicted: raw.severely_wounded_inflicted,
        severely_wounded_taken: raw.severely_wounded_taken,
        dps: rate_per_second(raw.damage_total, raw.battle_duration_total),
        sps: rate_per_second(raw.sps_total, raw.battle_duration_total),
        tps: rate_per_second(raw.tps_total, raw.battle_duration_total),
        hps: rate_per_second(raw.healing_total, raw.battle_duration_total),
    }
}

fn compute_trade_percentage(kill_points_gained: i64, kill_points_lost: i64) -> f64 {
    if kill_points_gained == kill_points_lost {
        100.0
    } else if kill_points_lost <= 0 {
        0.0
    } else {
        (kill_points_gained as f64 / kill_points_lost as f64) * 100.0
    }
}

fn divide(numerator: f64, denominator: f64) -> f64 {
    if denominator > 0.0 { numerator / denominator } else { 0.0 }
}

fn rate_per_second(total: i64, duration_millis: i64) -> f64 {
    divide(total as f64, duration_millis as f64 / 1000.0)
}

pub(super) fn build_precomputed_document(
    key: PairingKey,
    strategies: &PairingStrategies,
    drastc: Option<(&DrastcScore, &DrastcConfidence)>,
    refreshed_at: DateTime,
) -> Document {
    let all = strategies.all();
    let empty = StrategyRawTotals::default();
    let open_field = strategies.strategy(Strategy::OpenField).unwrap_or(&empty);
    let swarming = strategies.strategy(Strategy::Swarming).unwrap_or(&empty);
    let rally = strategies.strategy(Strategy::Rally).unwrap_or(&empty);
    let garrison = strategies.strategy(Strategy::Garrison).unwrap_or(&empty);

    doc! {
        "primary_commander_id": key.primary_commander_id,
        "secondary_commander_id": key.secondary_commander_id,
        "strategies": {
            "all": strategy_summary_document(&all),
            "open_field": strategy_summary_document(open_field),
            "swarming": strategy_summary_document(swarming),
            "rally": strategy_summary_document(rally),
            "garrison": strategy_summary_document(garrison),
        },
        "drastc": drastc.map(|(score, confidence)| drastc_score_document(score, confidence)),
        "refreshed_at": refreshed_at,
    }
}

fn strategy_summary_document(raw: &StrategyRawTotals) -> Document {
    let mut summary = summary_document(finalize_summary(raw.totals));
    summary.insert("power_loss_inflicted", raw.totals.power_loss_inflicted);
    summary.insert("power_loss_taken", raw.totals.power_loss_taken);
    summary.insert("atk_power_loss_inflicted", raw.totals.atk_power_loss_inflicted);
    summary.insert("atk_power_loss_taken", raw.totals.atk_power_loss_taken);
    summary.insert("skill_power_loss_inflicted", raw.totals.skill_power_loss_inflicted);
    summary.insert("skill_power_loss_taken", raw.totals.skill_power_loss_taken);
    summary.insert(
        "formations",
        raw.formations
            .iter()
            .map(|(formation, uses)| {
                Bson::Document(doc! {
                    "id": formation,
                    "count": uses,
                })
            })
            .collect::<Vec<_>>(),
    );
    summary
}

fn summary_document(summary: PairingSummary) -> Document {
    doc! {
        "total_battles": summary.total_battles,
        "kill_points_gained": summary.kill_points_gained,
        "kill_points_lost": summary.kill_points_lost,
        "avg_trade_percentage": summary.avg_trade_percentage,
        "weighted_trade_percentage": summary.weighted_trade_percentage,
        "avg_battle_duration": summary.avg_battle_duration,
        "total_battle_duration": summary.total_battle_duration,
        "severely_wounded_inflicted": summary.severely_wounded_inflicted,
        "severely_wounded_taken": summary.severely_wounded_taken,
        "dps": summary.dps,
        "sps": summary.sps,
        "tps": summary.tps,
        "hps": summary.hps,
    }
}

fn drastc_score_document(score: &DrastcScore, confidence: &DrastcConfidence) -> Document {
    doc! {
        "samples": u64_to_i64(score.samples),
        "breakdown": {
            "damage": category_score_document(score.breakdown.damage),
            "rage": category_score_document(score.breakdown.rage),
            "assist": category_score_document(score.breakdown.assist),
            "sustainability": category_score_document(score.breakdown.sustainability),
            "trade": category_score_document(score.breakdown.trade),
            "consistency": category_score_document(score.breakdown.consistency),
        },
        "overall": score.overall,
        "confidence": {
            "score": confidence.score,
            "unique_governors": u64_to_i64(confidence.unique_governors),
            "effective_governors": confidence.effective_governors,
        },
    }
}

fn category_score_document(score: rokbattles_drastc::CategoryScore) -> Document {
    doc! {
        "value": score.value,
        "p10": score.p10,
        "p90": score.p90,
        "score": score.score,
    }
}

fn u64_to_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use rokbattles_drastc::{
        BattleRecord, DrastcConfidence, DrastcModel, DrastcReferenceRanges, ReferenceRange,
    };

    use super::*;

    #[test]
    fn finalize_summary_computes_averages_and_rates() {
        let summary = finalize_summary(PairingRawTotals {
            total_battles: 2,
            kill_points_gained: 200,
            kill_points_lost: 100,
            trade_percentage_total: 300.0,
            battle_duration_total: 10_000,
            severely_wounded_inflicted: 20,
            severely_wounded_taken: 10,
            damage_total: 50,
            sps_total: 20,
            tps_total: 10,
            healing_total: 30,
            ..PairingRawTotals::default()
        });

        assert_eq!(summary.avg_trade_percentage, 150.0);
        assert_eq!(summary.weighted_trade_percentage, 200.0);
        assert_eq!(summary.avg_battle_duration, 5_000.0);
        assert_eq!(summary.dps, 5.0);
        assert_eq!(summary.hps, 3.0);
    }

    #[test]
    fn build_precomputed_document_emits_complete_zero_data_shape_for_unobserved_pairing() {
        let document = build_precomputed_document(
            PairingKey { primary_commander_id: 1, secondary_commander_id: 2 },
            &PairingStrategies::default(),
            None,
            DateTime::from_millis(0),
        );
        let zero_summary = doc! {
            "total_battles": 0_i64,
            "kill_points_gained": 0_i64,
            "kill_points_lost": 0_i64,
            "avg_trade_percentage": 0.0,
            "weighted_trade_percentage": 100.0,
            "avg_battle_duration": 0.0,
            "total_battle_duration": 0_i64,
            "severely_wounded_inflicted": 0_i64,
            "severely_wounded_taken": 0_i64,
            "dps": 0.0,
            "sps": 0.0,
            "tps": 0.0,
            "hps": 0.0,
        };
        let mut zero_strategy = zero_summary.clone();
        zero_strategy.insert("power_loss_inflicted", 0_i64);
        zero_strategy.insert("power_loss_taken", 0_i64);
        zero_strategy.insert("atk_power_loss_inflicted", 0_i64);
        zero_strategy.insert("atk_power_loss_taken", 0_i64);
        zero_strategy.insert("skill_power_loss_inflicted", 0_i64);
        zero_strategy.insert("skill_power_loss_taken", 0_i64);
        zero_strategy.insert("formations", Bson::Array(Vec::new()));

        assert_eq!(
            document,
            doc! {
                "primary_commander_id": 1_i64,
                "secondary_commander_id": 2_i64,
                "strategies": {
                    "all": zero_strategy.clone(),
                    "open_field": zero_strategy.clone(),
                    "swarming": zero_strategy.clone(),
                    "rally": zero_strategy.clone(),
                    "garrison": zero_strategy,
                },
                "drastc": Bson::Null,
                "refreshed_at": DateTime::from_millis(0),
            }
        );
    }

    #[test]
    fn compute_trade_percentage_handles_equal_zero_loss_and_weighted_cases() {
        assert_eq!(
            [
                compute_trade_percentage(0, 0),
                compute_trade_percentage(200, 0),
                compute_trade_percentage(200, 100),
            ],
            [100.0, 0.0, 200.0]
        );
    }

    #[test]
    fn build_precomputed_document_embeds_drastc_score_when_present() {
        let mut model = DrastcModel::new();
        model.set_reference_ranges(DrastcReferenceRanges {
            damage: ReferenceRange::new(1, 0.0, 4.0),
            sustainability: ReferenceRange::new(1, -2.0, 2.0),
            trade: ReferenceRange::new(1, 0.0, 2.0),
            consistency: ReferenceRange::new(1, 0.0, 1.0),
        });
        model.set_theoretical(579, 575);
        model.push(BattleRecord {
            sample_count: 1,
            total_duration_seconds: 100.0,
            kill_points: 200.0,
            opponent_kill_points: 100.0,
            opponent_dead: 10.0,
            opponent_severely_wounded: 20.0,
            opponent_slightly_wounded: 70.0,
            sender_dead: 0.0,
            sender_severely_wounded: 10.0,
            sender_slightly_wounded: 30.0,
            sender_healing: 5.0,
            decisive_battles: 1,
            wins: 1,
            positive_trades: 1,
        });
        let score = model.evaluate().expect("score");
        let confidence = DrastcConfidence::from_governor_distribution(1, 1, 1.0);

        let document = build_precomputed_document(
            PairingKey { primary_commander_id: 579, secondary_commander_id: 575 },
            &PairingStrategies::default(),
            Some((&score, &confidence)),
            DateTime::from_millis(0),
        );

        let drastc = document.get_document("drastc").expect("drastc document");
        assert_eq!(drastc.get_i64("samples").ok(), Some(1));
        let stored_confidence = drastc.get_document("confidence").expect("confidence document");
        assert_eq!(stored_confidence.get_i64("unique_governors"), Ok(1));
        assert_eq!(stored_confidence.get_f64("effective_governors"), Ok(1.0));
    }

    #[test]
    fn build_precomputed_document_emits_all_strategy_shapes() {
        let mut strategies = PairingStrategies::default();
        strategies.values.insert(
            Strategy::OpenField,
            StrategyRawTotals {
                totals: PairingRawTotals {
                    total_battles: 2,
                    kill_points_gained: 200,
                    kill_points_lost: 100,
                    trade_percentage_total: 300.0,
                    battle_duration_total: 10_000,
                    power_loss_inflicted: 1_200,
                    power_loss_taken: 900,
                    atk_power_loss_inflicted: 500,
                    atk_power_loss_taken: 400,
                    skill_power_loss_inflicted: 700,
                    skill_power_loss_taken: 500,
                    ..PairingRawTotals::default()
                },
                formations: BTreeMap::from([(0, 1), (1, 1)]),
            },
        );
        strategies.values.insert(
            Strategy::Rally,
            StrategyRawTotals {
                totals: PairingRawTotals {
                    total_battles: 3,
                    kill_points_gained: 300,
                    kill_points_lost: 200,
                    trade_percentage_total: 450.0,
                    battle_duration_total: 15_000,
                    power_loss_inflicted: 1_800,
                    power_loss_taken: 1_500,
                    atk_power_loss_inflicted: 800,
                    atk_power_loss_taken: 600,
                    skill_power_loss_inflicted: 1_000,
                    skill_power_loss_taken: 900,
                    ..PairingRawTotals::default()
                },
                formations: BTreeMap::from([(1, 2), (2, 1)]),
            },
        );

        let document = build_precomputed_document(
            PairingKey { primary_commander_id: 1, secondary_commander_id: 2 },
            &strategies,
            None,
            DateTime::from_millis(0),
        );
        let strategy_documents = document.get_document("strategies").expect("strategies");
        let all = strategy_documents.get_document("all").expect("all");

        assert_eq!(all.get_i64("total_battles"), Ok(5));
        assert_eq!(all.get_i64("power_loss_inflicted"), Ok(3_000));
        assert_eq!(all.get_i64("power_loss_taken"), Ok(2_400));
        assert_eq!(all.get_i64("atk_power_loss_inflicted"), Ok(1_300));
        assert_eq!(all.get_i64("atk_power_loss_taken"), Ok(1_000));
        assert_eq!(all.get_i64("skill_power_loss_inflicted"), Ok(1_700));
        assert_eq!(all.get_i64("skill_power_loss_taken"), Ok(1_400));
        for strategy in ["open_field", "swarming", "rally", "garrison"] {
            assert!(strategy_documents.get_document(strategy).is_ok());
        }
        assert_eq!(
            all.get_array("formations"),
            Ok(&vec![
                Bson::Document(doc! { "id": 0_i64, "count": 1_i64 }),
                Bson::Document(doc! { "id": 1_i64, "count": 3_i64 }),
                Bson::Document(doc! { "id": 2_i64, "count": 1_i64 }),
            ])
        );
    }
}
