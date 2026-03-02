use futures::FutureExt;
use futures::future::BoxFuture;
use mongodb::Collection;
use mongodb::IndexModel;
use mongodb::bson::{DateTime, doc};
use mongodb::options::{IndexOptions, UpdateOptions};
use serde::{Deserialize, Serialize};

/// Session data used by authenticated routes.
#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub session_id: String,
    pub user_id: String,
    pub created_at: DateTime,
    pub expires_at: DateTime,
}

/// User data used by authenticated routes.
#[derive(Debug, Clone)]
pub struct UserRecord {
    pub discord_id: String,
    pub email: String,
}

/// OAuth state persisted between login and callback.
#[derive(Debug, Clone)]
pub struct OAuthStateRecord {
    pub state: String,
    pub verifier: String,
    pub created_at: DateTime,
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
}

impl MongoAuthStore {
    /// Create a Mongo-backed auth repository.
    pub fn new(db: mongodb::Database) -> Self {
        Self {
            sessions: db.collection::<SessionDocument>("userSessions"),
            users: db.collection::<UserDocument>("users"),
            oauth_states: db.collection::<OAuthStateDocument>("oauthStates"),
        }
    }

    /// Ensure indexes for auth-related collections.
    pub async fn ensure_indexes(&self) -> mongodb::error::Result<()> {
        let session_indexes = vec![
            IndexModel::builder()
                .keys(doc! { "sessionId": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
            IndexModel::builder()
                .keys(doc! { "expiresAt": 1 })
                .options(
                    IndexOptions::builder()
                        .expire_after(Some(std::time::Duration::ZERO))
                        .build(),
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
                    IndexOptions::builder()
                        .expire_after(Some(std::time::Duration::ZERO))
                        .build(),
                )
                .build(),
        ];

        for index in oauth_indexes {
            self.oauth_states.create_index(index).await?;
        }

        Ok(())
    }
}

impl AuthRepository for MongoAuthStore {
    fn find_session_by_id<'a>(
        &'a self,
        sid: &'a str,
    ) -> BoxFuture<'a, Result<Option<SessionRecord>, AuthStoreError>> {
        async move {
            let doc = self
                .sessions
                .find_one(mongodb::bson::doc! { "sessionId": sid })
                .await?;
            Ok(doc.map(SessionRecord::from))
        }
        .boxed()
    }

    fn delete_session_by_id<'a>(
        &'a self,
        sid: &'a str,
    ) -> BoxFuture<'a, Result<(), AuthStoreError>> {
        async move {
            self.sessions
                .delete_one(mongodb::bson::doc! { "sessionId": sid })
                .await?;
            Ok(())
        }
        .boxed()
    }

    fn find_user_by_discord_id<'a>(
        &'a self,
        discord_id: &'a str,
    ) -> BoxFuture<'a, Result<Option<UserRecord>, AuthStoreError>> {
        async move {
            let doc = self
                .users
                .find_one(mongodb::bson::doc! { "discordId": discord_id })
                .await?;
            Ok(doc.map(UserRecord::from))
        }
        .boxed()
    }

    fn insert_oauth_state<'a>(
        &'a self,
        oauth_state: OAuthStateRecord,
    ) -> BoxFuture<'a, Result<(), AuthStoreError>> {
        async move {
            self.oauth_states
                .insert_one(OAuthStateDocument::from(oauth_state))
                .await?;
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
            let DiscordUserUpsert { discord_id, email } = user;
            let now = DateTime::now();
            self.users
                .update_one(
                    doc! { "discordId": &discord_id },
                    doc! {
                        "$set": {
                            "discordId": discord_id,
                            "email": email,
                            "updatedAt": now,
                        },
                        "$setOnInsert": {
                            "createdAt": now,
                        }
                    },
                )
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
            self.sessions
                .insert_one(SessionDocument::from(session))
                .await?;
            Ok(())
        }
        .boxed()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct SessionDocument {
    #[serde(rename = "sessionId")]
    session_id: String,
    #[serde(rename = "userId")]
    user_id: String,
    #[serde(rename = "createdAt")]
    created_at: DateTime,
    #[serde(rename = "expiresAt")]
    expires_at: DateTime,
}

impl From<SessionDocument> for SessionRecord {
    fn from(value: SessionDocument) -> Self {
        Self {
            session_id: value.session_id,
            user_id: value.user_id,
            created_at: value.created_at,
            expires_at: value.expires_at,
        }
    }
}

impl From<NewSessionRecord> for SessionDocument {
    fn from(value: NewSessionRecord) -> Self {
        Self {
            session_id: value.session_id,
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
}

impl From<UserDocument> for UserRecord {
    fn from(value: UserDocument) -> Self {
        Self {
            discord_id: value.discord_id,
            email: value.email,
        }
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
}

impl From<OAuthStateDocument> for OAuthStateRecord {
    fn from(value: OAuthStateDocument) -> Self {
        Self {
            state: value.state,
            verifier: value.verifier,
            created_at: value.created_at,
            expires_at: value.expires_at,
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
        }
    }
}
