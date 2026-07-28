//! Private learner note contracts.

use uuid::Uuid;

pub const MAX_NOTE_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateNote {
    pub subject_user_id: Uuid,
    pub journey_id: Uuid,
    pub activity_id: Option<Uuid>,
    pub content_version: Option<i32>,
    pub body: String,
    pub retry_key: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum NoteError {
    #[error("note field {field} must not be empty")]
    EmptyField { field: &'static str },
    #[error("note exceeds the 16 KiB limit")]
    TooLarge,
    #[error("note anchor is invalid")]
    InvalidAnchor,
    #[error("note retry key conflicts with another note")]
    RetryConflict,
    #[error("note not found")]
    NotFound,
    #[error("note storage failure: {0}")]
    Storage(String),
}

pub fn validate_note(input: &CreateNote) -> Result<(), NoteError> {
    if input.body.trim().is_empty() {
        return Err(NoteError::EmptyField { field: "body" });
    }
    if input.retry_key.trim().is_empty() {
        return Err(NoteError::EmptyField { field: "retry_key" });
    }
    if input.body.len() > MAX_NOTE_BYTES {
        return Err(NoteError::TooLarge);
    }
    if input.activity_id.is_some() != input.content_version.is_some()
        || input.content_version.is_some_and(|version| version < 1)
    {
        return Err(NoteError::InvalidAnchor);
    }
    Ok(())
}
