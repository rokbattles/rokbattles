use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use futures::{FutureExt, future::BoxFuture};
use mongodb::{
    Collection, IndexModel,
    bson::{DateTime, Document, doc},
    options::{IndexOptions, UpdateOptions},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::auth::Roles;

/// Session data used by authenticated routes.
#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub user_id: String,
    pub created_at: DateTime,
    pub expires_at: DateTime,
}

/// User data used by authenticated routes.
#[derive(Debug, Clone)]
pub struct UserRecord {
    pub discord_id: String,
    pub email: String,
    pub roles: Roles,
}

/// OAuth state persisted between login and callback.
#[derive(Debug, Clone)]
pub struct OAuthStateRecord {
    pub state: String,
    pub verifier: String,
    pub created_at: DateTime,
    pub expires_at: DateTime,
    pub desktop: Option<DesktopLogin>,
}

/// Loopback destination and PKCE binding for a desktop-initiated login.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DesktopLogin {
    pub port: u16,
    pub state: String,
    pub code_challenge: String,
}

/// Temporary desktop OAuth handoff state; the single-use code is stored only as a hash.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopOAuthState {
    pub code_hash: String,
    pub code_challenge: String,
    pub user_id: String,
    pub expires_at: DateTime,
}

/// User payload written during Discord OAuth callback.
#[derive(Debug, Clone)]
pub struct DiscordUserUpsert {
    pub discord_id: String,
    pub email: String,
}

/// Session payload written after successful login.
#[derive(Debug, Clone)]
pub struct NewSessionRecord {
    pub session_id: String,
    pub user_id: String,
    pub created_at: DateTime,
    pub expires_at: DateTime,
}

/// Error type for auth-store reads and writes.
#[derive(Debug, thiserror::Error)]
pub enum AuthStoreError {
    #[error("database error: {0}")]
    Database(#[from] mongodb::error::Error),
}

/// Auth-store interface so handlers are testable without MongoDB.
pub trait AuthRepository: Send + Sync {
    fn insert_desktop_oauth_state(
        &self,
        oauth_state: DesktopOAuthState,
    ) -> BoxFuture<'_, Result<(), AuthStoreError>>;

    /// Atomically consume an unexpired code only when its PKCE proof matches.
    fn consume_desktop_oauth_state<'a>(
        &'a self,
        code_hash: &'a str,
        code_challenge: &'a str,
        now: DateTime,
    ) -> BoxFuture<'a, Result<Option<DesktopOAuthState>, AuthStoreError>>;

    fn find_session_by_id<'a>(
        &'a self,
        sid: &'a str,
    ) -> BoxFuture<'a, Result<Option<SessionRecord>, AuthStoreError>>;

    fn delete_session_by_id<'a>(
        &'a self,
        sid: &'a str,
    ) -> BoxFuture<'a, Result<(), AuthStoreError>>;

    fn find_user_by_discord_id<'a>(
        &'a self,
        discord_id: &'a str,
    ) -> BoxFuture<'a, Result<Option<UserRecord>, AuthStoreError>>;

    fn insert_oauth_state<'a>(
        &'a self,
        oauth_state: OAuthStateRecord,
    ) -> BoxFuture<'a, Result<(), AuthStoreError>>;

    fn consume_oauth_state<'a>(
        &'a self,
        state: &'a str,
    ) -> BoxFuture<'a, Result<Option<OAuthStateRecord>, AuthStoreError>>;

    fn upsert_discord_user<'a>(
        &'a self,
        user: DiscordUserUpsert,
    ) -> BoxFuture<'a, Result<(), AuthStoreError>>;

    fn insert_session<'a>(
        &'a self,
        session: NewSessionRecord,
    ) -> BoxFuture<'a, Result<(), AuthStoreError>>;
}

#[derive(Debug, Clone)]
pub struct MongoAuthStore {
    sessions: Collection<SessionDocument>,
    users: Collection<UserDocument>,
    oauth_states: Collection<OAuthStateDocument>,
    desktop_oauth_states: Collection<DesktopOAuthState>,
}

impl MongoAuthStore {
    /// Create a Mongo-backed auth repository.
    pub fn new(db: mongodb::Database) -> Self {
        Self {
            sessions: db.collection::<SessionDocument>("userSessions"),
            users: db.collection::<UserDocument>("users"),
            oauth_states: db.collection::<OAuthStateDocument>("oauthStates"),
            desktop_oauth_states: db.collection("desktopOauthStates"),
        }
    }

    /// Ensure indexes for auth-related collections.
    pub async fn ensure_indexes(&self) -> mongodb::error::Result<()> {
        let session_indexes = vec![
            IndexModel::builder()
                .keys(doc! { "sessionHash": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
            IndexModel::builder()
                .keys(doc! { "expiresAt": 1 })
                .options(
                    IndexOptions::builder().expire_after(Some(std::time::Duration::ZERO)).build(),
                )
                .build(),
        ];

        for index in session_indexes {
            self.sessions.create_index(index).await?;
        }

        let user_index = IndexModel::builder()
            .keys(doc! { "discordId": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build();
        self.users.create_index(user_index).await?;

        let oauth_indexes = vec![
            IndexModel::builder()
                .keys(doc! { "state": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
            IndexModel::builder()
                .keys(doc! { "expiresAt": 1 })
                .options(
                    IndexOptions::builder().expire_after(Some(std::time::Duration::ZERO)).build(),
                )
                .build(),
        ];

        for index in oauth_indexes {
            self.oauth_states.create_index(index).await?;
        }

        self.desktop_oauth_states
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "codeHash": 1 })
                    .options(IndexOptions::builder().unique(true).build())
                    .build(),
            )
            .await?;

        self.desktop_oauth_states
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "expiresAt": 1 })
                    .options(
                        IndexOptions::builder()
                            .expire_after(Some(std::time::Duration::ZERO))
                            .build(),
                    )
                    .build(),
            )
            .await?;

        Ok(())
    }
}

impl AuthRepository for MongoAuthStore {
    fn insert_desktop_oauth_state(
        &self,
        oauth_state: DesktopOAuthState,
    ) -> BoxFuture<'_, Result<(), AuthStoreError>> {
        async move {
            self.desktop_oauth_states.insert_one(oauth_state).await?;
            Ok(())
        }
        .boxed()
    }

    fn consume_desktop_oauth_state<'a>(
        &'a self,
        code_hash: &'a str,
        code_challenge: &'a str,
        now: DateTime,
    ) -> BoxFuture<'a, Result<Option<DesktopOAuthState>, AuthStoreError>> {
        async move {
            Ok(self
                .desktop_oauth_states
                .find_one_and_delete(desktop_oauth_consume_filter(code_hash, code_challenge, now))
                .await?)
        }
        .boxed()
    }

    fn find_session_by_id<'a>(
        &'a self,
        sid: &'a str,
    ) -> BoxFuture<'a, Result<Option<SessionRecord>, AuthStoreError>> {
        async move {
            let document =
                self.sessions.find_one(doc! { "sessionHash": hash_session_id(sid) }).await?;
            Ok(document.map(SessionRecord::from))
        }
        .boxed()
    }

    fn delete_session_by_id<'a>(
        &'a self,
        sid: &'a str,
    ) -> BoxFuture<'a, Result<(), AuthStoreError>> {
        async move {
            self.sessions.delete_one(doc! { "sessionHash": hash_session_id(sid) }).await?;
            Ok(())
        }
        .boxed()
    }

    fn find_user_by_discord_id<'a>(
        &'a self,
        discord_id: &'a str,
    ) -> BoxFuture<'a, Result<Option<UserRecord>, AuthStoreError>> {
        async move {
            let doc = self.users.find_one(mongodb::bson::doc! { "discordId": discord_id }).await?;
            Ok(doc.map(UserRecord::from))
        }
        .boxed()
    }

    fn insert_oauth_state<'a>(
        &'a self,
        oauth_state: OAuthStateRecord,
    ) -> BoxFuture<'a, Result<(), AuthStoreError>> {
        async move {
            self.oauth_states.insert_one(OAuthStateDocument::from(oauth_state)).await?;
            Ok(())
        }
        .boxed()
    }

    fn consume_oauth_state<'a>(
        &'a self,
        state: &'a str,
    ) -> BoxFuture<'a, Result<Option<OAuthStateRecord>, AuthStoreError>> {
        async move {
            let doc = self
                .oauth_states
                .find_one_and_delete(mongodb::bson::doc! { "state": state })
                .await?;
            Ok(doc.map(OAuthStateRecord::from))
        }
        .boxed()
    }

    fn upsert_discord_user<'a>(
        &'a self,
        user: DiscordUserUpsert,
    ) -> BoxFuture<'a, Result<(), AuthStoreError>> {
        async move {
            let now = DateTime::now();
            self.users
                .update_one(doc! { "discordId": &user.discord_id }, discord_user_update(&user, now))
                .with_options(UpdateOptions::builder().upsert(true).build())
                .await?;
            Ok(())
        }
        .boxed()
    }

    fn insert_session<'a>(
        &'a self,
        session: NewSessionRecord,
    ) -> BoxFuture<'a, Result<(), AuthStoreError>> {
        async move {
            self.sessions.insert_one(SessionDocument::from(session)).await?;
            Ok(())
        }
        .boxed()
    }
}

fn discord_user_update(user: &DiscordUserUpsert, now: DateTime) -> Document {
    doc! {
        "$set": {
            "discordId": &user.discord_id,
            "email": &user.email,
            "updatedAt": now,
        },
        "$setOnInsert": {
            "createdAt": now,
        }
    }
}

fn desktop_oauth_consume_filter(code_hash: &str, code_challenge: &str, now: DateTime) -> Document {
    doc! {
        "codeHash": code_hash,
        "codeChallenge": code_challenge,
        "expiresAt": { "$gt": now },
    }
}

fn hash_session_id(sid: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(sid.as_bytes()))
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct SessionDocument {
    #[serde(rename = "sessionHash")]
    session_hash: String,
    #[serde(rename = "userId")]
    user_id: String,
    #[serde(rename = "createdAt")]
    created_at: DateTime,
    #[serde(rename = "expiresAt")]
    expires_at: DateTime,
}

impl From<SessionDocument> for SessionRecord {
    fn from(value: SessionDocument) -> Self {
        Self { user_id: value.user_id, created_at: value.created_at, expires_at: value.expires_at }
    }
}

impl From<NewSessionRecord> for SessionDocument {
    fn from(value: NewSessionRecord) -> Self {
        Self {
            session_hash: hash_session_id(&value.session_id),
            user_id: value.user_id,
            created_at: value.created_at,
            expires_at: value.expires_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct UserDocument {
    #[serde(rename = "discordId")]
    discord_id: String,
    email: String,
    #[serde(default)]
    roles: Roles,
}

impl From<UserDocument> for UserRecord {
    fn from(value: UserDocument) -> Self {
        Self { discord_id: value.discord_id, email: value.email, roles: value.roles }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct OAuthStateDocument {
    state: String,
    verifier: String,
    #[serde(rename = "createdAt")]
    created_at: DateTime,
    #[serde(rename = "expiresAt")]
    expires_at: DateTime,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    desktop: Option<DesktopLogin>,
}

impl From<OAuthStateDocument> for OAuthStateRecord {
    fn from(value: OAuthStateDocument) -> Self {
        Self {
            state: value.state,
            verifier: value.verifier,
            created_at: value.created_at,
            expires_at: value.expires_at,
            desktop: value.desktop,
        }
    }
}

impl From<OAuthStateRecord> for OAuthStateDocument {
    fn from(value: OAuthStateRecord) -> Self {
        Self {
            state: value.state,
            verifier: value.verifier,
            created_at: value.created_at,
            expires_at: value.expires_at,
            desktop: value.desktop,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::Role;

    #[test]
    fn oauth_updates_only_identity_fields_and_preserves_role_assignments() {
        let update = discord_user_update(
            &DiscordUserUpsert {
                discord_id: "user".to_string(),
                email: "new@example.com".to_string(),
            },
            DateTime::now(),
        );

        // Restrict both fields and operators: a replacement or unset could remove
        // role assignments without assigning a new roles value.
        for (operator, fields) in &update {
            assert!(matches!(operator.as_str(), "$set" | "$setOnInsert"));

            for field in fields.as_document().expect("update fields").keys() {
                assert!(matches!(
                    field.as_str(),
                    "discordId" | "email" | "updatedAt" | "createdAt"
                ));
            }
        }
    }

    #[test]
    fn legacy_users_have_only_the_implicit_user_role() {
        for roles in
            [None, Some(mongodb::bson::Bson::Null), Some(mongodb::bson::Bson::Array(vec![]))]
        {
            let mut document = doc! { "discordId": "user", "email": "user@example.com" };

            if let Some(roles) = roles {
                document.insert("roles", roles);
            }

            let user = UserRecord::from(
                mongodb::bson::from_document::<UserDocument>(document).expect("legacy user"),
            );

            assert_eq!(user.roles, Roles::default());
            assert!(user.roles.contains(Role::User));
        }
    }

    #[test]
    fn stored_roles_are_independent_and_deduplicated() {
        let document: UserDocument = mongodb::bson::from_document(doc! {
            "discordId": "user", "email": "user@example.com",
            "roles": ["content_creator", "developer_program", "content_creator", "user"],
        })
        .expect("user roles");

        let user = UserRecord::from(document);

        assert_eq!(
            serde_json::to_value(user.roles).expect("effective roles"),
            serde_json::json!(["user", "content_creator", "developer_program"]),
        );
    }

    #[test]
    fn malformed_or_unknown_role_data_fails_closed() {
        for roles in [
            mongodb::bson::Bson::from("admin"),
            mongodb::bson::Bson::from(1),
            mongodb::bson::Bson::from(doc! { "admin": true }),
            mongodb::bson::Bson::Array(vec!["admin".into(), 1.into()]),
            mongodb::bson::Bson::Array(vec!["admin".into(), "unknown_role".into()]),
            mongodb::bson::Bson::Array(vec!["Admin".into()]),
            mongodb::bson::Bson::Array(vec![doc! { "admin": mongodb::bson::Bson::Null }.into()]),
            mongodb::bson::Bson::Array(vec![mongodb::bson::Bson::Null]),
        ] {
            mongodb::bson::from_document::<UserDocument>(doc! {
                "discordId": "user", "email": "user@example.com", "roles": roles,
            })
            .expect_err("malformed roles must reject the entire record");
        }
    }

    #[test]
    fn desktop_consume_query_requires_code_pkce_and_strict_expiry_together() {
        let now = DateTime::from_millis(1_700_000_000_000);
        let filter = desktop_oauth_consume_filter("code-hash", "pkce-challenge", now);

        assert_eq!(filter.get_str("codeHash").expect("code binding"), "code-hash");
        assert_eq!(filter.get_str("codeChallenge").expect("PKCE binding"), "pkce-challenge");
        assert_eq!(
            filter.get_document("expiresAt").expect("expiry predicate"),
            &doc! { "$gt": now }
        );
    }

    #[test]
    fn session_hash_matches_sha256_base64url_vector() {
        assert_eq!(hash_session_id("abc"), "ungWv48Bz-pBQUDeXa4iI7ADYaOWF3qctBD_YfIAFa0");
    }

    #[test]
    fn serialized_session_contains_hash_instead_of_bearer_token() {
        let document = mongodb::bson::to_document(&SessionDocument::from(NewSessionRecord {
            session_id: "secret-token".to_string(),
            user_id: "discord-user".to_string(),
            created_at: DateTime::from_millis(1000),
            expires_at: DateTime::from_millis(2000),
        }))
        .expect("serialize session");

        assert!(!document.contains_key("sessionId"));
        assert_eq!(document.get_str("sessionHash").expect("hash"), hash_session_id("secret-token"));
    }

    #[test]
    fn preexisting_website_oauth_states_do_not_require_desktop_metadata() {
        let document: OAuthStateDocument = mongodb::bson::from_document(doc! {
            "state": "existing-state", "verifier": "existing-verifier",
            "createdAt": DateTime::now(), "expiresAt": DateTime::now(),
        })
        .expect("legacy OAuth state");
        let record = OAuthStateRecord::from(document);

        assert!(record.desktop.is_none());
        assert_eq!(record.verifier, "existing-verifier");
    }
}
