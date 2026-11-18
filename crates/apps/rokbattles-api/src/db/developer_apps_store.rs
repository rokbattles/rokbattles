//! MongoDB storage for registered developer apps.

use futures::TryStreamExt;
use mongodb::{
    Collection, Database, IndexModel,
    bson::{DateTime, Document, doc, oid::ObjectId, to_document},
    options::ReturnDocument,
};
use serde::{Deserialize, Serialize};

pub(crate) const DEVELOPER_APPS_COLLECTION: &str = "v3_developer_apps";

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DeveloperAppSettings {
    pub name: String,
    pub description: Option<String>,
    pub terms_of_service_url: Option<String>,
    pub privacy_policy_url: Option<String>,
    #[serde(default)]
    pub redirect_uris: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DeveloperAppGeneralSettings {
    pub name: String,
    pub description: Option<String>,
    pub terms_of_service_url: Option<String>,
    pub privacy_policy_url: Option<String>,
}

impl DeveloperAppGeneralSettings {
    pub(crate) fn with_redirect_uris(self, redirect_uris: Vec<String>) -> DeveloperAppSettings {
        DeveloperAppSettings {
            name: self.name,
            description: self.description,
            terms_of_service_url: self.terms_of_service_url,
            privacy_policy_url: self.privacy_policy_url,
            redirect_uris,
        }
    }
}

pub(crate) enum DeveloperAppUpdate {
    All(DeveloperAppSettings),
    General(DeveloperAppGeneralSettings),
    RedirectUris(Vec<String>),
}

impl DeveloperAppUpdate {
    fn into_fields(self) -> mongodb::bson::ser::Result<Document> {
        Ok(match self {
            Self::All(settings) => doc! { "settings": to_document(&settings)? },
            Self::General(general) => doc! {
                "settings.name": general.name,
                "settings.description": general.description,
                "settings.termsOfServiceUrl": general.terms_of_service_url,
                "settings.privacyPolicyUrl": general.privacy_policy_url,
            },
            Self::RedirectUris(uris) => doc! { "settings.redirectUris": uris },
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeveloperApp {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub terms_accepted_at: DateTime,
    pub settings: DeveloperAppSettings,
}

/// Persists developer app registrations and their owner-managed settings.
#[derive(Clone)]
pub struct DeveloperAppsStore {
    collection: Collection<DeveloperApp>,
}

impl DeveloperAppsStore {
    pub fn new(db: Database) -> Self {
        Self { collection: db.collection(DEVELOPER_APPS_COLLECTION) }
    }

    pub async fn ensure_indexes(&self) -> mongodb::error::Result<()> {
        self.collection
            .create_index(IndexModel::builder().keys(doc! { "userId": 1, "_id": -1 }).build())
            .await?;
        Ok(())
    }

    pub(crate) async fn list(&self, user_id: &str) -> mongodb::error::Result<Vec<DeveloperApp>> {
        self.collection
            .find(doc! { "userId": user_id })
            .sort(doc! { "_id": -1 })
            .await?
            .try_collect()
            .await
    }

    pub(crate) async fn find(
        &self,
        user_id: &str,
        id: ObjectId,
    ) -> mongodb::error::Result<Option<DeveloperApp>> {
        self.collection.find_one(owned_app(user_id, id)).await
    }

    pub(crate) async fn insert(
        &self,
        user_id: &str,
        settings: DeveloperAppSettings,
    ) -> mongodb::error::Result<DeveloperApp> {
        let terms_accepted_at = DateTime::now();
        let document = doc! {
            "userId": user_id,
            "termsAcceptedAt": terms_accepted_at,
            "settings": to_document(&settings)?,
        };

        let result = self.collection.clone_with_type::<Document>().insert_one(document).await?;
        let id = result
            .inserted_id
            .as_object_id()
            .ok_or_else(|| mongodb::error::Error::custom("missing developer app ObjectId"))?;

        Ok(DeveloperApp { id, terms_accepted_at, settings })
    }

    pub(crate) async fn update(
        &self,
        user_id: &str,
        id: ObjectId,
        update: DeveloperAppUpdate,
    ) -> mongodb::error::Result<Option<DeveloperApp>> {
        self.collection
            .find_one_and_update(owned_app(user_id, id), doc! { "$set": update.into_fields()? })
            .return_document(ReturnDocument::After)
            .await
    }
}

fn owned_app(user_id: &str, id: ObjectId) -> Document {
    doc! { "_id": id, "userId": user_id }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_updates_target_only_their_own_fields() {
        let general = DeveloperAppGeneralSettings {
            name: "Roster Tool".into(),
            description: None,
            terms_of_service_url: Some("https://example.org/terms".into()),
            privacy_policy_url: None,
        };

        assert_eq!(
            DeveloperAppUpdate::General(general).into_fields().unwrap(),
            doc! {
                "settings.name": "Roster Tool",
                "settings.description": null,
                "settings.termsOfServiceUrl": "https://example.org/terms",
                "settings.privacyPolicyUrl": null,
            }
        );
        assert_eq!(
            DeveloperAppUpdate::RedirectUris(vec![]).into_fields().unwrap(),
            doc! { "settings.redirectUris": [] }
        );
    }
}
