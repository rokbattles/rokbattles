use futures::FutureExt;
use futures::future::BoxFuture;
use mongodb::Collection;
use mongodb::bson::DateTime;
use serde::Deserialize;

/// Session fields used by authenticated API handlers.
#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub session_id: String,
    pub user_id: String,
    pub created_at: DateTime,
    pub expires_at: DateTime,
}

/// User fields used by authenticated API handlers.
#[derive(Debug, Clone)]
pub struct UserRecord {
    pub discord_id: String,
    pub username: String,
    pub global_name: Option<String>,
    pub email: String,
    pub avatar: Option<String>,
}

/// Storage error for session/user lookups.
#[derive(Debug, thiserror::Error)]
pub enum AuthStoreError {
    #[error("database error: {0}")]
    Database(#[from] mongodb::error::Error),
}

/// Abstract auth storage so handlers can be tested without MongoDB.
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
}

#[derive(Debug, Clone)]
pub struct MongoAuthStore {
    sessions: Collection<SessionDocument>,
    users: Collection<UserDocument>,
}

impl MongoAuthStore {
    /// Create a Mongo-backed auth repository.
    pub fn new(db: mongodb::Database) -> Self {
        Self {
            sessions: db.collection::<SessionDocument>("userSessions"),
            users: db.collection::<UserDocument>("users"),
        }
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
}

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
struct UserDocument {
    #[serde(rename = "discordId")]
    discord_id: String,
    username: String,
    #[serde(rename = "globalName")]
    global_name: Option<String>,
    email: String,
    avatar: Option<String>,
}

impl From<UserDocument> for UserRecord {
    fn from(value: UserDocument) -> Self {
        Self {
            discord_id: value.discord_id,
            username: value.username,
            global_name: value.global_name,
            email: value.email,
            avatar: value.avatar,
        }
    }
}
