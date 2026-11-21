//! MongoDB storage for program applications.
//!
//! A partial unique index limits each account to one open application per program.
//! Closing an application releases that slot and keeps the submitted answers.

use futures::TryStreamExt;
use mongodb::{
    Collection, IndexModel,
    bson::{DateTime, Document, doc, oid::ObjectId, to_bson},
    options::{IndexOptions, ReturnDocument},
};
use serde::{Deserialize, Serialize};

use super::auth_store::USERS_COLLECTION;
use crate::auth::Role;

/// Name of the collection that stores program applications.
pub const PROGRAM_APPLICATIONS_COLLECTION: &str = "v3_program_applications";

/// A program that accepts user applications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Program {
    ContentCreator,
    DeveloperProgram,
}

impl Program {
    /// Returns the role granted to approved applicants.
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

/// A question and its submitted answer.
///
/// Keeping the original question lets reviewers read the answer in context
/// after the form changes.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FormAnswer {
    pub question: String,
    pub answer: String,
}

/// A program application and its review state.
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
    /// Whether an approval is reserved while the application is still open.
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
    /// Creates a store for the given database.
    pub fn new(db: mongodb::Database) -> Self {
        Self {
            collection: db.collection(PROGRAM_APPLICATIONS_COLLECTION),
            users: db.collection(USERS_COLLECTION),
        }
    }

    /// Creates the indexes for open submissions and newest-first reviews.
    ///
    /// # Errors
    ///
    /// Returns an error if MongoDB cannot create the indexes. Existing duplicate
    /// open applications prevent the unique index from being created.
    pub async fn ensure_indexes(&self) -> mongodb::error::Result<()> {
        let newest_first = IndexModel::builder().keys(doc! { "createdAt": -1, "_id": -1 }).build();

        self.collection.create_indexes([open_application_index(), newest_first]).await?;

        Ok(())
    }

    /// Lists an account's open applications across both programs.
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

    /// Lists applications newest first, leaving form answers for the detail query.
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

    /// Inserts a validated application.
    ///
    /// # Errors
    ///
    /// Returns a duplicate-key error if the account already has an open
    /// application for this program. Other insert failures come from MongoDB.
    pub async fn insert(&self, application: &ProgramApplication) -> mongodb::error::Result<()> {
        self.collection.insert_one(application).await?;

        Ok(())
    }

    /// Reserves an open application for approval.
    ///
    /// The application stays open until the role is granted so it still blocks
    /// another submission. A reviewer can retry an interrupted approval.
    /// Denial is allowed only before the reservation is made.
    ///
    /// Returns `false` if the application is missing or no longer open.
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

    /// Grants the program role if the account does not already have it.
    ///
    /// A missing or null roles field is treated as an empty list. Returns
    /// `false` if the account is missing.
    pub(crate) async fn grant_role(
        &self,
        application: &ProgramApplication,
    ) -> mongodb::error::Result<bool> {
        let role = to_bson(&application.program.role())?;
        let update = vec![doc! {
            "$set": {
                "roles": {
                    "$let": {
                        "vars": { "current_roles": { "$ifNull": ["$roles", []] } },
                        "in": {
                            "$cond": [
                                { "$in": [role.clone(), "$$current_roles"] },
                                "$$current_roles",
                                { "$concatArrays": ["$$current_roles", [role]] },
                            ],
                        },
                    },
                },
            },
        }];

        let result =
            self.users.update_one(doc! { "discordId": &application.user_id }, update).await?;

        Ok(result.matched_count == 1)
    }

    /// Marks a reserved application as approved after granting its program role.
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

    /// Closes an open application that has no pending approval.
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
