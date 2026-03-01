use futures::TryStreamExt;
use mongodb::Collection;
use mongodb::bson::Document;
use mongodb::options::FindOptions;

use crate::error::ApiError;

/// Fetch projected Mongo documents and deserialize them into `T`.
pub(crate) async fn fetch_collection_documents<T>(
    collection: &Collection<Document>,
    filter: Document,
    options: FindOptions,
) -> Result<Vec<T>, ApiError>
where
    T: serde::de::DeserializeOwned + Send + Sync,
{
    let typed_collection = collection.clone_with_type::<T>();
    let cursor = typed_collection
        .find(filter)
        .with_options(options)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    cursor
        .try_collect()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))
}
