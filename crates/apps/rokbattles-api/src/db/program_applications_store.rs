//! MongoDB storage for program applications.
//!
//! Each user can have one open application per program. A partial unique index
//! enforces this limit; closed applications do not prevent later submissions.

use futures::TryStreamExt;
use mongodb::{
    Collection, IndexModel,
    bson::{DateTime, doc, oid::ObjectId},
    options::IndexOptions,
};
use serde::{Deserialize, Serialize};

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
    pub form: Vec<FormAnswer>,
    pub status: String,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

/// Stores program applications in MongoDB.
#[derive(Clone)]
pub struct ProgramApplicationsStore {
    collection: Collection<ProgramApplication>,
}

impl ProgramApplicationsStore {
    /// Creates a store backed by the given database.
    pub fn new(db: mongodb::Database) -> Self {
        Self { collection: db.collection(PROGRAM_APPLICATIONS_COLLECTION) }
    }

    /// Ensures that a user can have at most one open application per program.
    ///
    /// # Errors
    ///
    /// Returns an error if MongoDB cannot create the index, including when
    /// existing documents violate the uniqueness constraint.
    pub async fn ensure_indexes(&self) -> mongodb::error::Result<()> {
        self.collection.create_index(open_application_index()).await?;

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
