use futures::StreamExt;
use mongodb::{
    Collection,
    bson::{Bson, Document},
};
use rokbattles_bson::{bson_to_f64, bson_to_i64};
use rokbattles_drastc::{DrastcReferenceRanges, ReferenceRange};

use super::{
    model::{
        PairingKey, PairingRawTotals, PairingStrategies, PairingsAggregation, Strategy,
        StrategyRawTotals,
    },
    pipeline::build_pairings_pipeline,
};
use crate::error::JobsError;

pub(super) async fn read_pairings_and_reference_ranges(
    source: &Collection<Document>,
    legendary_ids: &[i64],
) -> Result<PairingsAggregation, JobsError> {
    let pipeline = build_pairings_pipeline(legendary_ids);
    let mut cursor = source.aggregate(pipeline).allow_disk_use(true).await?;
    let mut aggregation = PairingsAggregation::default();

    while let Some(next) = cursor.next().await {
        merge_pairings_result_document(&mut aggregation, &next?);
    }

    Ok(aggregation)
}

fn map_raw_totals_document(document: &Document) -> PairingRawTotals {
    PairingRawTotals {
        total_battles: direct_i64(document, "total_battles").unwrap_or_default(),
        kill_points_gained: direct_i64(document, "kill_points_gained").unwrap_or_default(),
        kill_points_lost: direct_i64(document, "kill_points_lost").unwrap_or_default(),
        trade_percentage_total: direct_f64(document, "trade_percentage_total").unwrap_or_default(),
        battle_duration_total: direct_i64(document, "battle_duration_total").unwrap_or_default(),
        severely_wounded_inflicted: direct_i64(document, "severely_wounded_inflicted")
            .unwrap_or_default(),
        severely_wounded_taken: direct_i64(document, "severely_wounded_taken").unwrap_or_default(),
        power_loss_inflicted: direct_i64(document, "power_loss_inflicted").unwrap_or_default(),
        power_loss_taken: direct_i64(document, "power_loss_taken").unwrap_or_default(),
        atk_power_loss_inflicted: direct_i64(document, "atk_power_loss_inflicted")
            .unwrap_or_default(),
        atk_power_loss_taken: direct_i64(document, "atk_power_loss_taken").unwrap_or_default(),
        skill_power_loss_inflicted: direct_i64(document, "skill_power_loss_inflicted")
            .unwrap_or_default(),
        skill_power_loss_taken: direct_i64(document, "skill_power_loss_taken").unwrap_or_default(),
        damage_total: direct_i64(document, "damage_total").unwrap_or_default(),
        sps_total: direct_i64(document, "sps_total").unwrap_or_default(),
        tps_total: direct_i64(document, "tps_total").unwrap_or_default(),
        healing_total: direct_i64(document, "healing_total").unwrap_or_default(),
        opponent_dead: direct_i64(document, "opponent_dead").unwrap_or_default(),
        opponent_slightly_wounded: direct_i64(document, "opponent_slightly_wounded")
            .unwrap_or_default(),
        sender_dead: direct_i64(document, "sender_dead").unwrap_or_default(),
        sender_slightly_wounded: direct_i64(document, "sender_slightly_wounded")
            .unwrap_or_default(),
        normalized_duration_seconds_total: direct_f64(
            document,
            "normalized_duration_seconds_total",
        )
        .unwrap_or_default(),
        decisive_battles: direct_i64(document, "decisive_battles").unwrap_or_default(),
        wins: direct_i64(document, "wins").unwrap_or_default(),
        positive_trades: direct_i64(document, "positive_trades").unwrap_or_default(),
    }
}

#[cfg(test)]
fn map_pairings_result_document(document: &Document) -> PairingsAggregation {
    let mut aggregation = PairingsAggregation::default();
    merge_pairings_result_document(&mut aggregation, document);
    aggregation
}

fn merge_pairings_result_document(aggregation: &mut PairingsAggregation, document: &Document) {
    let Some(key) = direct_i64(document, "primary_commander_id")
        .zip(direct_i64(document, "secondary_commander_id"))
        .map(|(primary_commander_id, secondary_commander_id)| PairingKey {
            primary_commander_id,
            secondary_commander_id,
        })
    else {
        return;
    };

    let mut strategies = PairingStrategies::default();
    if let Ok(documents) = document.get_array("strategies") {
        for document in documents.iter().filter_map(Bson::as_document) {
            let Some(strategy) = document.get_str("strategy").ok().and_then(Strategy::from_str)
            else {
                continue;
            };
            let formations = document
                .get_array("formations")
                .ok()
                .into_iter()
                .flatten()
                .filter_map(Bson::as_document)
                .filter_map(|formation| {
                    direct_i64(formation, "id").zip(direct_i64(formation, "count"))
                })
                .collect();
            strategies.values.insert(
                strategy,
                StrategyRawTotals { totals: map_raw_totals_document(document), formations },
            );
        }
    }
    aggregation.strategies.insert(key, strategies);

    if let Ok(document) = document.get_document("drastc_observed") {
        aggregation.drastc_observed.insert(key, map_raw_totals_document(document));
    }

    if let Ok(document) = document.get_document("reference_ranges")
        && let Some(reference_ranges) = map_reference_ranges_document(document)
    {
        aggregation.reference_ranges = reference_ranges;
    }
}

fn map_reference_ranges_document(document: &Document) -> Option<DrastcReferenceRanges> {
    let samples = usize::try_from(direct_i64(document, "samples")?).ok()?;

    Some(DrastcReferenceRanges {
        damage: reference_range_from_percentiles(samples, document, "damage"),
        sustainability: reference_range_from_percentiles(samples, document, "sustainability"),
        trade: trade_reference_range_from_percentiles(samples, document),
        consistency: reference_range_from_percentiles(samples, document, "consistency"),
    })
}

fn trade_reference_range_from_percentiles(samples: usize, document: &Document) -> ReferenceRange {
    let Some(Bson::Array(values)) = document.get("trade") else {
        return ReferenceRange::new(0, 0.0, 0.0);
    };

    ReferenceRange::new(samples, 0.0, values.first().and_then(bson_to_f64).unwrap_or_default())
}

fn reference_range_from_percentiles(
    samples: usize,
    document: &Document,
    key: &str,
) -> ReferenceRange {
    let Some(Bson::Array(values)) = document.get(key) else {
        return ReferenceRange::new(0, 0.0, 0.0);
    };

    ReferenceRange::new(
        samples,
        values.first().and_then(bson_to_f64).unwrap_or_default(),
        values.get(1).and_then(bson_to_f64).unwrap_or_default(),
    )
}

fn direct_i64(document: &Document, key: &str) -> Option<i64> {
    document.get(key).and_then(bson_to_i64)
}

fn direct_f64(document: &Document, key: &str) -> Option<f64> {
    document.get(key).and_then(bson_to_f64)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use mongodb::bson::doc;

    use super::*;

    #[test]
    fn map_pairings_result_document_accumulates_strategies_and_keeps_open_field_for_drastc() {
        let result = map_pairings_result_document(&doc! {
            "primary_commander_id": 579_i64,
            "secondary_commander_id": 575_i64,
            "strategies": [{
                "total_battles": 2_i64,
                "strategy": "open_field",
                "formations": [{ "id": 0_i64, "count": 2_i64 }],
            }, {
                "total_battles": 3_i64,
                "strategy": "rally",
                "formations": [{ "id": 2_i64, "count": 3_i64 }],
            }],
            "drastc_observed": {
                "total_battles": 2_i64,
            },
            "reference_ranges": {
                "samples": 1_i64,
                "damage": [1.0, 2.0],
                "sustainability": [1.0, 2.0],
                "consistency": [0.1, 0.9],
                "trade": [2.0],
            },
        });
        let pairing = PairingKey { primary_commander_id: 579, secondary_commander_id: 575 };
        let strategies = result.strategies.get(&pairing).expect("strategies");

        assert_eq!(strategies.all().totals.total_battles, 5);
        assert_eq!(strategies.all().formations, BTreeMap::from([(0, 2), (2, 3)]));
        assert_eq!(
            result.drastc_observed.get(&pairing).map(|totals| totals.total_battles),
            Some(2)
        );
    }

    #[test]
    fn map_raw_totals_document_maps_numeric_totals() {
        let document = doc! {
            "primary_commander_id": 509_i64,
            "secondary_commander_id": 6_i64,
            "total_battles": 2_i64,
            "kill_points_gained": 200_i64,
            "kill_points_lost": 100_i64,
            "trade_percentage_total": 300.0,
            "battle_duration_total": 10_000_i64,
            "severely_wounded_inflicted": 20_i64,
            "severely_wounded_taken": 10_i64,
            "power_loss_inflicted": 1_200_i64,
            "power_loss_taken": 900_i64,
            "atk_power_loss_inflicted": 500_i64,
            "atk_power_loss_taken": 400_i64,
            "skill_power_loss_inflicted": 700_i64,
            "skill_power_loss_taken": 500_i64,
            "damage_total": 50_i64,
            "sps_total": 20_i64,
            "tps_total": 10_i64,
            "healing_total": 30_i64,
            "opponent_dead": 5_i64,
            "opponent_slightly_wounded": 30_i64,
            "sender_dead": 2_i64,
            "sender_slightly_wounded": 15_i64,
            "normalized_duration_seconds_total": 10.0,
            "decisive_battles": 1_i64,
            "wins": 1_i64,
            "positive_trades": 1_i64,
        };

        let totals = map_raw_totals_document(&document);
        assert_eq!(totals.total_battles, 2);
        assert_eq!(totals.power_loss_inflicted, 1_200);
        assert_eq!(totals.power_loss_taken, 900);
        assert_eq!(totals.atk_power_loss_inflicted, 500);
        assert_eq!(totals.atk_power_loss_taken, 400);
        assert_eq!(totals.skill_power_loss_inflicted, 700);
        assert_eq!(totals.skill_power_loss_taken, 500);
        assert_eq!(totals.damage_total, 50);
        assert_eq!(totals.opponent_dead, 5);
        assert_eq!(totals.normalized_duration_seconds_total, 10.0);
        assert_eq!(totals.positive_trades, 1);
    }

    #[test]
    fn map_reference_ranges_document_maps_percentile_arrays() {
        let document = doc! {
            "samples": 10_i64,
            "damage": [1.0, 9.0],
            "sustainability": [-5.0, 5.0],
            "trade": [1.8],
            "consistency": [0.2, 0.8],
        };

        let ranges = map_reference_ranges_document(&document).expect("reference ranges");

        assert_eq!(ranges.damage.p10, 1.0);
        assert_eq!(ranges.damage.p90, 9.0);
        assert_eq!(ranges.damage.sample_count(), 10);
        assert_eq!(ranges.trade.p10, 0.0);
        assert_eq!(ranges.trade.p90, 1.8);
        assert_eq!(ranges.trade.sample_count(), 10);
    }
}
