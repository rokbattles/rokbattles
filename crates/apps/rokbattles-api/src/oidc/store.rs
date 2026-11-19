//! Stores grants and single-use credentials. MongoDB conditional writes enforce
//! one-time consumption across API instances, including on standalone MongoDB.

use std::time::Duration;

use async_trait::async_trait;
use futures::TryStreamExt;
use mongodb::{
    Collection, Database, IndexModel,
    bson::{DateTime, Document, doc, oid::ObjectId, to_bson},
    error::Result,
    options::{IndexOptions, ReturnDocument},
};
use serde::{Deserialize, Serialize};

use super::{Scope, random_token};
use crate::db::{DeveloperApp, DeveloperAppsStore, is_duplicate_key};

pub(crate) const GRANTS_COLLECTION: &str = "v3_oidc_grants";

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Request {
    #[serde(rename = "_id")]
    pub(super) id: String,
    pub(super) browser_hash: String,
    pub(super) client_id: ObjectId,
    pub(super) redirect_uri: String,
    pub(super) state: String,
    pub(super) nonce: String,
    pub(super) challenge: String,
    pub(super) scopes: Vec<Scope>,
    pub(super) binding: Option<BrowserBinding>,
    pub(super) expires_at: DateTime,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BrowserBinding {
    pub(super) session_id: ObjectId,
    pub(super) subject: String,
    pub(super) grant_version: Option<ObjectId>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Grant {
    #[serde(rename = "_id")]
    pub(super) id: String,
    pub(super) generation: ObjectId,
    pub(super) version: ObjectId,
    pub(super) client_id: ObjectId,
    pub(super) owner_id: String,
    pub(super) user_id: String,
    pub(super) scopes: Vec<Scope>,
    pub(super) authorized_on: DateTime,
    pub(super) revoked: bool,
}

pub(super) fn grant_id(subject: &str, client_id: ObjectId) -> String {
    format!("{subject}.{}", client_id.to_hex())
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Authorization {
    pub(super) client_id: ObjectId,
    pub(super) user_id: String,
    pub(super) subject: String,
    pub(super) session_id: ObjectId,
    pub(super) grant_generation: ObjectId,
    pub(super) scopes: Vec<Scope>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Code {
    #[serde(rename = "_id")]
    pub(super) hash: String,
    pub(super) authorization: Authorization,
    pub(super) redirect_uri: String,
    pub(super) challenge: String,
    pub(super) nonce: String,
    pub(super) expires_at: DateTime,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AccessToken {
    pub(super) hash: String,
    pub(super) scopes: Vec<Scope>,
    pub(super) expires_at: DateTime,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SpentRefreshToken {
    pub(super) hash: String,
    pub(super) expires_at: DateTime,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RefreshFamily {
    #[serde(rename = "_id")]
    pub(super) id: ObjectId,
    pub(super) authorization: Authorization,
    pub(super) current_hash: String,
    pub(super) spent_hashes: Vec<String>,
    #[serde(default)]
    pub(super) spent_refresh_tokens: Vec<SpentRefreshToken>,
    pub(super) access_tokens: Vec<AccessToken>,
    pub(super) refresh_expires_at: DateTime,
    pub(super) expires_at: DateTime,
    pub(super) revoked: bool,
    // Existing families keep their session check until their next successful rotation.
    #[serde(default)]
    pub(super) rolling: bool,
}

#[async_trait]
pub(super) trait OidcRepository: Send + Sync {
    async fn registration(&self, id: ObjectId) -> Result<Option<DeveloperApp>>;
    async fn subject(&self, user_id: &str) -> Result<Option<String>>;
    async fn insert_request(&self, request: &Request) -> Result<()>;
    async fn request(&self, id: &str, browser_hash: &str, now: DateTime)
    -> Result<Option<Request>>;
    async fn bind_request(
        &self,
        id: &str,
        browser_hash: &str,
        binding: &BrowserBinding,
        now: DateTime,
    ) -> Result<Option<Request>>;
    async fn cancel_unbound_request(
        &self,
        id: &str,
        browser_hash: &str,
        now: DateTime,
    ) -> Result<bool>;
    async fn consume_request(
        &self,
        id: &str,
        browser_hash: &str,
        session_id: ObjectId,
        now: DateTime,
    ) -> Result<bool>;
    async fn grant(&self, id: &str) -> Result<Option<Grant>>;
    async fn save_grant(&self, next: &Grant, previous: Option<ObjectId>) -> Result<bool>;
    async fn list_grants(&self, user_id: &str) -> Result<Vec<Grant>>;
    async fn revoke_grant(&self, id: &str, user_id: &str) -> Result<bool>;
    async fn insert_code(&self, code: &Code) -> Result<()>;
    async fn code(&self, hash: &str, now: DateTime) -> Result<Option<Code>>;
    async fn consume_code(&self, hash: &str, now: DateTime) -> Result<bool>;
    async fn insert_family(&self, family: &RefreshFamily) -> Result<()>;
    async fn family(&self, id: ObjectId) -> Result<Option<RefreshFamily>>;
    async fn rotate_family(&self, next: &RefreshFamily, current_hash: &str) -> Result<bool>;
    async fn revoke_replayed_family(
        &self,
        id: ObjectId,
        spent_hash: &str,
        now: DateTime,
    ) -> Result<()>;
}

pub(super) struct MongoOidcStore {
    users: Collection<Document>,
    apps: DeveloperAppsStore,
    requests: Collection<Request>,
    grants: Collection<Grant>,
    codes: Collection<Code>,
    families: Collection<RefreshFamily>,
}

impl MongoOidcStore {
    pub(super) fn new(db: Database) -> Self {
        Self {
            users: db.collection("users"),
            apps: DeveloperAppsStore::new(db.clone()),
            requests: db.collection("v3_oidc_requests"),
            grants: db.collection(GRANTS_COLLECTION),
            codes: db.collection("v3_oidc_codes"),
            families: db.collection("v3_oidc_token_families"),
        }
    }

    pub(super) async fn ensure_indexes(&self) -> Result<()> {
        let ttl = IndexModel::builder()
            .keys(doc! { "expiresAt": 1 })
            .options(IndexOptions::builder().expire_after(Duration::ZERO).build())
            .build();

        self.requests.create_index(ttl.clone()).await?;
        self.codes.create_index(ttl.clone()).await?;
        self.families.create_index(ttl).await?;
        self.grants
            .create_indexes([
                IndexModel::builder().keys(doc! { "userId": 1, "revoked": 1 }).build(),
                IndexModel::builder().keys(doc! { "ownerId": 1 }).build(),
            ])
            .await?;

        Ok(())
    }
}

#[async_trait]
impl OidcRepository for MongoOidcStore {
    async fn registration(&self, id: ObjectId) -> Result<Option<DeveloperApp>> {
        self.apps.find_registration(id).await
    }

    async fn subject(&self, user_id: &str) -> Result<Option<String>> {
        let filter = doc! { "discordId": user_id };
        let projection = doc! { "oidcSubject": 1 };
        let document = self.users.find_one(filter.clone()).projection(projection.clone()).await?;
        if let Some(subject) =
            document.as_ref().and_then(|record| record.get_str("oidcSubject").ok())
        {
            return Ok(Some(subject.to_owned()));
        }
        if document.is_none() {
            return Ok(None);
        }

        // Concurrent first-time requests must all observe the same subject.
        let document = self
            .users
            .find_one_and_update(
                filter,
                vec![doc! { "$set": {
                    "oidcSubject": { "$ifNull": ["$oidcSubject", random_token()] }
                } }],
            )
            .projection(projection)
            .return_document(ReturnDocument::After)
            .await?;

        Ok(document.and_then(|record| record.get_str("oidcSubject").ok().map(str::to_owned)))
    }

    async fn insert_request(&self, request: &Request) -> Result<()> {
        self.requests.insert_one(request).await?;
        Ok(())
    }

    async fn request(
        &self,
        id: &str,
        browser_hash: &str,
        now: DateTime,
    ) -> Result<Option<Request>> {
        self.requests.find_one(request_filter(id, browser_hash, now)).await
    }

    async fn bind_request(
        &self,
        id: &str,
        browser_hash: &str,
        binding: &BrowserBinding,
        now: DateTime,
    ) -> Result<Option<Request>> {
        let mut filter = request_filter(id, browser_hash, now);
        filter.insert("binding", mongodb::bson::Bson::Null);
        let bound = self
            .requests
            .find_one_and_update(filter, doc! { "$set": { "binding": to_bson(binding)? } })
            .return_document(ReturnDocument::After)
            .await?;

        if bound.is_some() {
            return Ok(bound);
        }

        let mut filter = request_filter(id, browser_hash, now);
        filter.insert("binding.sessionId", binding.session_id);
        self.requests.find_one(filter).await
    }

    async fn cancel_unbound_request(
        &self,
        id: &str,
        browser_hash: &str,
        now: DateTime,
    ) -> Result<bool> {
        let mut filter = request_filter(id, browser_hash, now);
        filter.insert("binding", mongodb::bson::Bson::Null);
        Ok(self.requests.delete_one(filter).await?.deleted_count == 1)
    }

    async fn consume_request(
        &self,
        id: &str,
        browser_hash: &str,
        session_id: ObjectId,
        now: DateTime,
    ) -> Result<bool> {
        let mut filter = request_filter(id, browser_hash, now);
        filter.insert("binding.sessionId", session_id);
        Ok(self.requests.delete_one(filter).await?.deleted_count == 1)
    }

    async fn grant(&self, id: &str) -> Result<Option<Grant>> {
        self.grants.find_one(doc! { "_id": id }).await
    }

    async fn save_grant(&self, next: &Grant, previous: Option<ObjectId>) -> Result<bool> {
        if let Some(version) = previous {
            let filter = doc! { "_id": &next.id, "version": version };
            return Ok(self.grants.replace_one(filter, next).await?.matched_count == 1);
        }

        match self.grants.insert_one(next).await {
            Ok(_) => Ok(true),
            Err(error) if is_duplicate_key(&error) => Ok(false),
            Err(error) => Err(error),
        }
    }

    async fn list_grants(&self, user_id: &str) -> Result<Vec<Grant>> {
        self.grants
            .find(doc! { "userId": user_id, "revoked": false })
            .sort(doc! { "authorizedOn": -1 })
            .await?
            .try_collect()
            .await
    }

    async fn revoke_grant(&self, id: &str, user_id: &str) -> Result<bool> {
        let result = self
            .grants
            .update_one(
                doc! { "_id": id, "userId": user_id, "revoked": false },
                doc! { "$set": { "revoked": true, "version": ObjectId::new() } },
            )
            .await?;
        Ok(result.matched_count == 1)
    }

    async fn insert_code(&self, code: &Code) -> Result<()> {
        self.codes.insert_one(code).await?;
        Ok(())
    }

    async fn code(&self, hash: &str, now: DateTime) -> Result<Option<Code>> {
        self.codes.find_one(doc! { "_id": hash, "expiresAt": { "$gt": now } }).await
    }

    async fn consume_code(&self, hash: &str, now: DateTime) -> Result<bool> {
        let result =
            self.codes.delete_one(doc! { "_id": hash, "expiresAt": { "$gt": now } }).await?;
        Ok(result.deleted_count == 1)
    }

    async fn insert_family(&self, family: &RefreshFamily) -> Result<()> {
        self.families.insert_one(family).await?;
        Ok(())
    }

    async fn family(&self, id: ObjectId) -> Result<Option<RefreshFamily>> {
        self.families.find_one(doc! { "_id": id }).await
    }

    async fn rotate_family(&self, next: &RefreshFamily, current_hash: &str) -> Result<bool> {
        let filter = doc! { "_id": next.id, "currentHash": current_hash, "revoked": false };
        Ok(self.families.replace_one(filter, next).await?.matched_count == 1)
    }

    async fn revoke_replayed_family(
        &self,
        id: ObjectId,
        spent_hash: &str,
        now: DateTime,
    ) -> Result<()> {
        self.families
            .update_one(
                doc! {
                    "_id": id,
                    "expiresAt": { "$gt": now },
                    "refreshExpiresAt": { "$gt": now },
                    "$or": [
                        { "spentHashes": spent_hash },
                        { "spentRefreshTokens": { "$elemMatch": {
                            "hash": spent_hash,
                            "expiresAt": { "$gt": now },
                        } } },
                    ],
                },
                doc! { "$set": { "revoked": true } },
            )
            .await?;
        Ok(())
    }
}

fn request_filter(id: &str, browser_hash: &str, now: DateTime) -> Document {
    doc! { "_id": id, "browserHash": browser_hash, "expiresAt": { "$gt": now } }
}
