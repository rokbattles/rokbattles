use futures::StreamExt;
use mongodb::{
    Collection,
    bson::{Bson, Document, doc},
    options::Hint,
};
use rokbattles_bson::{bson_to_f64, bson_to_i64};
use rokbattles_drastc::{DrastcReferenceRanges, ReferenceRange};

use super::{
    model::{DrastcAggregation, PairingKey, PairingRawTotals},
    pipeline::build_drastc_pipeline,
};
use crate::error::JobsError;

pub(super) async fn read_drastc_aggregation(
    source: &Collection<Document>,
    legendary_ids: &[i64],
    cutoff_mail_time: i64,
) -> Result<DrastcAggregation, JobsError> {
    let mut cursor = source
        .aggregate(build_drastc_pipeline(legendary_ids, cutoff_mail_time))
        .allow_disk_use(true)
        .hint(Hint::Keys(
            doc! { "metadata.mail_time": -1, "metadata.kvk": 1, "opponents.player_id": 1 },
        ))
        .await?;
    let mut aggregation = DrastcAggregation::default();
    while let Some(next) = cursor.next().await {
        merge_result(&mut aggregation, &next?);
    }
    Ok(aggregation)
}

fn merge_result(aggregation: &mut DrastcAggregation, document: &Document) {
    let Some(key) = direct_i64(document, "primary_commander_id")
        .zip(direct_i64(document, "secondary_commander_id"))
        .map(|(primary_commander_id, secondary_commander_id)| PairingKey {
            primary_commander_id,
            secondary_commander_id,
        })
    else {
        return;
    };

    if let Ok(observed) = document.get_document("observed") {
        aggregation.observed.insert(key, map_raw_totals(observed));
    }
    if let Ok(ranges) = document.get_document("reference_ranges")
        && let Some(ranges) = map_reference_ranges(ranges)
    {
        aggregation.reference_ranges = ranges;
    }
}

fn map_raw_totals(document: &Document) -> PairingRawTotals {
    PairingRawTotals {
        total_battles: direct_i64(document, "total_battles").unwrap_or_default(),
        kill_points_gained: direct_i64(document, "kill_points_gained").unwrap_or_default(),
        kill_points_lost: direct_i64(document, "kill_points_lost").unwrap_or_default(),
        severely_wounded_inflicted: direct_i64(document, "severely_wounded_inflicted")
            .unwrap_or_default(),
        severely_wounded_taken: direct_i64(document, "severely_wounded_taken").unwrap_or_default(),
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

fn map_reference_ranges(document: &Document) -> Option<DrastcReferenceRanges> {
    let samples = usize::try_from(direct_i64(document, "samples")?).ok()?;
    Some(DrastcReferenceRanges {
        damage: percentile_range(samples, document, "damage"),
        sustainability: percentile_range(samples, document, "sustainability"),
        trade: trade_range(samples, document),
        consistency: percentile_range(samples, document, "consistency"),
    })
}

fn percentile_range(samples: usize, document: &Document, key: &str) -> ReferenceRange {
    let Some(Bson::Array(values)) = document.get(key) else {
        return ReferenceRange::new(0, 0.0, 0.0);
    };
    ReferenceRange::new(
        samples,
        values.first().and_then(bson_to_f64).unwrap_or_default(),
        values.get(1).and_then(bson_to_f64).unwrap_or_default(),
    )
}

fn trade_range(samples: usize, document: &Document) -> ReferenceRange {
    let Some(Bson::Array(values)) = document.get("trade") else {
        return ReferenceRange::new(0, 0.0, 0.0);
    };
    ReferenceRange::new(samples, 0.0, values.first().and_then(bson_to_f64).unwrap_or_default())
}

fn direct_i64(document: &Document, key: &str) -> Option<i64> {
    document.get(key).and_then(bson_to_i64)
}
fn direct_f64(document: &Document, key: &str) -> Option<f64> {
    document.get(key).and_then(bson_to_f64)
}

#[cfg(test)]
mod tests {
    use mongodb::bson::doc;

    use super::*;

    #[test]
    fn merge_result_maps_observed_totals_and_reference_ranges() {
        let mut result = DrastcAggregation::default();
        merge_result(
            &mut result,
            &doc! {
                "primary_commander_id": 579_i64,
                "secondary_commander_id": 575_i64,
                "observed": { "total_battles": 2_i64, "kill_points_gained": 250_i64 },
                "reference_ranges": { "samples": 3_i64, "damage": [1.0, 2.0], "sustainability": [-1.0, 1.0], "consistency": [0.1, 0.9], "trade": [2.0] },
            },
        );
        let key = PairingKey { primary_commander_id: 579, secondary_commander_id: 575 };
        assert_eq!(result.observed[&key].total_battles, 2);
        assert_eq!(result.reference_ranges.damage.sample_count(), 3);
    }
}
