use std::sync::Arc;

use mongodb::bson::{Bson, Document, doc};
use mongodb::options::FindOptions;
use serde::Deserialize;

use crate::error::ApiError;
use crate::routes::governor::store_utils::fetch_collection_documents;
use crate::state::AppState;

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct MailMetadataDocument {
    #[serde(default)]
    pub mail_time: Option<Bson>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct RssSectionDocument {
    #[serde(default)]
    pub rss_type: Option<Bson>,
    #[serde(default)]
    pub rss_value: Option<Bson>,
    #[serde(default)]
    pub rss_bonus: Option<Bson>,
    #[serde(default)]
    pub crystals_gain: Option<Bson>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct RssMailDocument {
    #[serde(default)]
    pub metadata: Option<MailMetadataDocument>,
    #[serde(default)]
    pub rss: Option<RssSectionDocument>,
}

pub(crate) async fn fetch_resources_mails(
    state: &Arc<AppState>,
    mail_receiver: &str,
    time_match: &Document,
) -> Result<Vec<RssMailDocument>, ApiError> {
    let options = FindOptions::builder()
        .projection(doc! {
            "_id": 0,
            "metadata.mail_time": 1,
            "rss.rss_type": 1,
            "rss.rss_value": 1,
            "rss.rss_bonus": 1,
            "rss.crystals_gain": 1,
        })
        .build();

    let filter = doc! {
        "$and": [
            { "metadata.mail_receiver": mail_receiver },
            time_match.clone(),
        ]
    };

    fetch_collection_documents(state.reports_store.rss_collection(), filter, options).await
}
