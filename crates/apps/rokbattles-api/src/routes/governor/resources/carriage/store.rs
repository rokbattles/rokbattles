//! Reads processed carriage reports and normalizes values for aggregation.

use mongodb::{
    bson::{Bson, doc},
    options::FindOptions,
};
use rokbattles_bson::{bson_to_f64, bson_to_i64_exact, bson_to_i64_loose};
use serde::Deserialize;

use super::aggregate::{Direction, ResourceAmount, TransferReport};
use crate::{
    error::ApiError,
    routes::governor::{date_range::GovernorDateRange, store_utils::fetch_collection_documents},
    state::AppState,
    time_utils::normalize_bson_timestamp_millis,
};

#[derive(Deserialize)]
struct CarriageDocument {
    metadata: Metadata,
    carriage: Carriage,
}

#[derive(Deserialize)]
struct Metadata {
    mail_id: String,
    mail_time: Bson,
}

#[derive(Deserialize)]
struct Carriage {
    target_id: i64,
    target_name: String,
    active: bool,
    resources: Vec<Resource>,
    tax: Option<Bson>,
}

#[derive(Deserialize)]
struct Resource {
    #[serde(rename = "type")]
    kind: i64,
    value: Bson,
}

/// Returns the governor's reports within the requested UTC date range.
pub(super) async fn fetch_reports(
    state: &AppState,
    governor_id: i64,
    range: Option<&GovernorDateRange>,
) -> Result<Vec<TransferReport>, ApiError> {
    let filter = doc! { "metadata.mail_receiver": format!("player_{governor_id}") };
    let options = FindOptions::builder()
        .projection(doc! {
            "_id": 0,
            "metadata.mail_id": 1,
            "metadata.mail_time": 1,
            "carriage.target_id": 1,
            "carriage.target_name": 1,
            "carriage.active": 1,
            "carriage.resources": 1,
            "carriage.tax": 1,
        })
        .build();

    let documents: Vec<CarriageDocument> =
        fetch_collection_documents(state.reports_store.carriage_collection(), filter, options)
            .await?;

    Ok(documents.into_iter().filter_map(|document| document.into_transfer_report(range)).collect())
}

impl CarriageDocument {
    fn into_transfer_report(self, range: Option<&GovernorDateRange>) -> Option<TransferReport> {
        let Self { metadata, carriage } = self;

        // Apply date bounds after normalization so the stored timestamp unit
        // does not change which reports are included.
        let timestamp_millis = normalize_bson_timestamp_millis(Some(&metadata.mail_time))?;
        if let Some(range) = range
            && !(range.start_millis..range.end_millis).contains(&timestamp_millis)
        {
            return None;
        }

        let raw_timestamp = bson_to_i64_loose(&metadata.mail_time).unwrap_or(timestamp_millis);
        let resources = carriage
            .resources
            .into_iter()
            .filter(|resource| matches!(resource.kind, 1..=4))
            .filter_map(|resource| {
                let amount = bson_to_i64_exact(&resource.value).filter(|amount| *amount > 0)?;
                Some(ResourceAmount { kind: resource.kind, amount })
            })
            .collect();

        // The processor uses zero when tax is absent.
        let tax =
            carriage.tax.as_ref().and_then(bson_to_f64).filter(|rate| *rate > 0.0 && *rate <= 1.0);

        Some(TransferReport {
            mail_id: metadata.mail_id,
            timestamp_millis,
            raw_timestamp,
            governor_id: carriage.target_id,
            name: carriage.target_name,
            direction: if carriage.active { Direction::Sent } else { Direction::Received },
            resources,
            tax,
        })
    }
}
