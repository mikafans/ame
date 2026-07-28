//! Learner note repository port.

use crate::domain::note::{CreateNote, NoteError};
use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub id: Uuid,
    pub subject_user_id: Uuid,
    pub journey_id: Uuid,
    pub activity_id: Option<Uuid>,
    pub content_version: Option<i32>,
    pub body: String,
    pub revision: i32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[async_trait]
pub trait NoteRepository: Send + Sync {
    async fn create(&self, input: CreateNote) -> Result<Note, NoteError>;
    async fn list(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
        activity_id: Option<Uuid>,
    ) -> Result<Vec<Note>, NoteError>;
    async fn update(
        &self,
        subject_user_id: Uuid,
        id: Uuid,
        expected_revision: i32,
        body: String,
    ) -> Result<Note, NoteError>;
    async fn delete(&self, subject_user_id: Uuid, id: Uuid) -> Result<(), NoteError>;
}
