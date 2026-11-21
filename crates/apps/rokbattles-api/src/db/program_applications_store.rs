//! MongoDB storage for program applications.
//!
//! Each user can have one open application per program. A partial unique index
//! enforces this limit; closed applications do not prevent later submissions.

use futures::TryStreamExt;
use mongodb::{
    Collection, IndexModel,
    bson::{DateTime, Document, doc, oid::ObjectId, to_bson},
    options::{IndexOptions, ReturnDocument},
};
use serde::{Deserialize, Serialize};

use super::auth_store::USERS_COLLECTION;
use crate::auth::Role;

/// Collection containing applications for both programs.
pub const PROGRAM_APPLICATIONS_COLLECTION: &str = "v3_program_applications";

/// A program that accepts user applications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Program {
    ContentCreator,
    DeveloperProgram,
}

impl Program {
    /// Returns the site role granted when this program's application is approved.
    pub fn role(self) -> Role {
        match self {
            Self::ContentCreator => Role::ContentCreator,
            Self::DeveloperProgram => Role::DeveloperProgram,
        }
    }

    /// Returns the question recorded with a new application for this program.
    pub fn question(self) -> &'static str {
        match self {
            Self::ContentCreator => "YouTube Channel URL",
            Self::DeveloperProgram => "What would you like to build with ROK Battles?",
        }
    }
}

/// The question and validated answer recorded when an application is submitted.
///
/// Storing the question preserves its wording if the form changes later.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FormAnswer {
    pub question: String,
    pub answer: String,
}

/// A stored application belonging to a Discord user.
///
/// New applications have status `open` and identical creation and update times.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgramApplication {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub user_id: String,
    pub program: Program,
    #[serde(default)]
    pub form: Vec<FormAnswer>,
    pub status: String,
    #[serde(default)]
    pub approval_started: bool,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

/// Stores program applications in MongoDB.
#[derive(Clone)]
pub struct ProgramApplicationsStore {
    collection: Collection<ProgramApplication>,
    users: Collection<Document>,
}

impl ProgramApplicationsStore {
    /// Creates a store backed by the given database.
    pub fn new(db: mongodb::Database) -> Self {
        Self {
            collection: db.collection(PROGRAM_APPLICATIONS_COLLECTION),
            users: db.collection(USERS_COLLECTION),
        }
    }

    /// Ensures unique open applications and supports newest-first pagination.
    ///
    /// # Errors
    ///
    /// Returns an error if MongoDB cannot create the index, including when
    /// existing documents violate the uniqueness constraint.
    pub async fn ensure_indexes(&self) -> mongodb::error::Result<()> {
        self.collection
            .create_indexes([
                open_application_index(),
                IndexModel::builder().keys(doc! { "createdAt": -1, "_id": -1 }).build(),
            ])
            .await?;

        Ok(())
    }

    /// Lists the user's open applications across both programs.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails or a document cannot be deserialized.
    pub async fn list_open(
        &self,
        user_id: &str,
    ) -> mongodb::error::Result<Vec<ProgramApplication>> {
        self.collection
            .find(doc! { "userId": user_id, "status": "open" })
            .await?
            .try_collect()
            .await
    }

    /// Inserts a validated application.
    ///
    /// # Errors
    ///
    /// Returns a duplicate-key error if another open application exists for the
    /// same user and program, or another database error if the insertion fails.
    pub async fn insert(&self, application: &ProgramApplication) -> mongodb::error::Result<()> {
        self.collection.insert_one(application).await?;

        Ok(())
    }

    pub(crate) async fn list_for_review(
        &self,
        offset: u64,
        limit: i64,
    ) -> mongodb::error::Result<Vec<ProgramApplication>> {
        self.collection
            .find(doc! {})
            .sort(doc! { "createdAt": -1, "_id": -1 })
            .skip(offset)
            .limit(limit)
            .projection(doc! { "form": 0 })
            .await?
            .try_collect()
            .await
    }

    pub(crate) async fn find_by_id(
        &self,
        id: ObjectId,
    ) -> mongodb::error::Result<Option<ProgramApplication>> {
        self.collection.find_one(doc! { "_id": id }).await
    }

    /// Reserve approval before granting the role, including on standalone MongoDB.
    /// Keeping the application open preserves its uniqueness constraint. If either
    /// write fails, another approval can resume it and denial cannot race it.
    pub(crate) async fn start_approval(&self, id: ObjectId) -> mongodb::error::Result<bool> {
        let result = self
            .collection
            .update_one(
                doc! { "_id": id, "status": "open" },
                doc! { "$set": { "approvalStarted": true } },
            )
            .await?;
        Ok(result.matched_count == 1)
    }

    /// Adds the program role only if it is absent. Accounts with a missing or
    /// null roles field start with an empty list.
    pub(crate) async fn grant_role(
        &self,
        application: &ProgramApplication,
    ) -> mongodb::error::Result<bool> {
        let role = to_bson(&application.program.role())?;
        let result = self
            .users
            .update_one(
                doc! { "discordId": &application.user_id },
                vec![doc! {
                    "$set": { "roles": {
                        "$let": {
                            "vars": { "current": { "$ifNull": ["$roles", []] } },
                            "in": { "$cond": [
                                { "$in": [role.clone(), "$$current"] },
                                "$$current",
                                { "$concatArrays": ["$$current", [role]] },
                            ] },
                        }
                    } }
                }],
            )
            .await?;
        Ok(result.matched_count == 1)
    }

    pub(crate) async fn complete_approval(
        &self,
        id: ObjectId,
    ) -> mongodb::error::Result<Option<ProgramApplication>> {
        self.collection
            .find_one_and_update(
                doc! { "_id": id, "status": "open", "approvalStarted": true },
                doc! {
                    "$set": { "status": "approved" },
                    "$unset": { "approvalStarted": "" },
                    "$currentDate": { "updatedAt": true },
                },
            )
            .return_document(ReturnDocument::After)
            .await
    }

    pub(crate) async fn deny(
        &self,
        id: ObjectId,
    ) -> mongodb::error::Result<Option<ProgramApplication>> {
        self.collection
            .find_one_and_update(
                doc! { "_id": id, "status": "open", "approvalStarted": { "$ne": true } },
                doc! {
                    "$set": { "status": "closed" },
                    "$currentDate": { "updatedAt": true },
                },
            )
            .return_document(ReturnDocument::After)
            .await
    }
}

fn open_application_index() -> IndexModel {
    IndexModel::builder()
        .keys(doc! { "userId": 1, "program": 1 })
        .options(
            IndexOptions::builder()
                .unique(true)
                .partial_filter_expression(doc! { "status": "open" })
                .build(),
        )
        .build()
}
