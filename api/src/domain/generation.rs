use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum GenerationStatus {
    Requested,
    Running,
    ReviewRequired,
    Published,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartGenerationRun {
    pub subject_user_id: Uuid,
    pub source_actor_id: Uuid,
    pub operation: String,
    pub provider: Option<String>,
    pub retry_key: Option<String>,
    pub content_version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationRun {
    pub id: Uuid,
    pub subject_user_id: Uuid,
    pub source_actor_id: Uuid,
    pub operation: String,
    pub provider: Option<String>,
    pub retry_key: Option<String>,
    pub content_version: u32,
    pub status: GenerationStatus,
    pub error: Option<Value>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[async_trait]
pub trait GenerationRepository: Send + Sync {
    async fn start(&self, input: StartGenerationRun) -> Result<GenerationRun, GenerationError>;

    async fn transition(
        &self,
        subject_user_id: Uuid,
        id: Uuid,
        status: GenerationStatus,
        error: Option<Value>,
    ) -> Result<GenerationRun, GenerationError>;

    async fn get(&self, subject_user_id: Uuid, id: Uuid) -> Result<GenerationRun, GenerationError>;
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum GenerationError {
    #[error("generation field {field} must not be empty")]
    EmptyField { field: &'static str },
    #[error("generation content version must be greater than zero")]
    InvalidContentVersion,
    #[error("invalid generation transition from {from:?} to {to:?}")]
    InvalidTransition {
        from: GenerationStatus,
        to: GenerationStatus,
    },
    #[error("generation run belongs to another subject")]
    SubjectMismatch,
    #[error("generation run was not found")]
    NotFound,
    #[error("generation storage failure: {0}")]
    Storage(String),
}

pub fn validate_start(input: &StartGenerationRun) -> Result<(), GenerationError> {
    if input.operation.trim().is_empty() {
        return Err(GenerationError::EmptyField { field: "operation" });
    }
    if input
        .provider
        .as_deref()
        .is_some_and(|provider| provider.trim().is_empty())
    {
        return Err(GenerationError::EmptyField { field: "provider" });
    }
    if input
        .retry_key
        .as_deref()
        .is_some_and(|retry_key| retry_key.trim().is_empty())
    {
        return Err(GenerationError::EmptyField { field: "retry_key" });
    }
    if input.content_version == 0 {
        return Err(GenerationError::InvalidContentVersion);
    }
    Ok(())
}

pub fn transition_status(
    from: GenerationStatus,
    to: GenerationStatus,
) -> Result<GenerationStatus, GenerationError> {
    let allowed = matches!(
        (from, to),
        (GenerationStatus::Requested, GenerationStatus::Running)
            | (GenerationStatus::Requested, GenerationStatus::Failed)
            | (GenerationStatus::Running, GenerationStatus::ReviewRequired)
            | (GenerationStatus::Running, GenerationStatus::Published)
            | (GenerationStatus::Running, GenerationStatus::Failed)
            | (
                GenerationStatus::ReviewRequired,
                GenerationStatus::Published
            )
            | (GenerationStatus::ReviewRequired, GenerationStatus::Failed)
    );
    allowed
        .then_some(to)
        .ok_or(GenerationError::InvalidTransition { from, to })
}

#[cfg(test)]
mod tests {
    use super::{GenerationStatus, StartGenerationRun, transition_status, validate_start};
    use uuid::Uuid;

    #[test]
    fn generation_lifecycle_allows_publishable_and_failure_paths() {
        assert_eq!(
            transition_status(GenerationStatus::Requested, GenerationStatus::Running),
            Ok(GenerationStatus::Running)
        );
        assert_eq!(
            transition_status(GenerationStatus::Running, GenerationStatus::ReviewRequired),
            Ok(GenerationStatus::ReviewRequired)
        );
        assert_eq!(
            transition_status(GenerationStatus::Running, GenerationStatus::Published),
            Ok(GenerationStatus::Published)
        );
        assert_eq!(
            transition_status(GenerationStatus::Running, GenerationStatus::Failed),
            Ok(GenerationStatus::Failed)
        );
    }

    #[test]
    fn generation_lifecycle_rejects_false_publication_and_illegal_retries() {
        assert!(transition_status(GenerationStatus::Failed, GenerationStatus::Published).is_err());
        assert!(transition_status(GenerationStatus::Published, GenerationStatus::Running).is_err());
        assert!(
            transition_status(
                GenerationStatus::ReviewRequired,
                GenerationStatus::Requested
            )
            .is_err()
        );
    }

    #[test]
    fn generation_start_requires_meaningful_retry_metadata() {
        let input = StartGenerationRun {
            subject_user_id: Uuid::new_v4(),
            source_actor_id: Uuid::new_v4(),
            operation: "question.compose".into(),
            provider: Some("external-agent".into()),
            retry_key: Some("attempt-1".into()),
            content_version: 1,
        };
        assert!(validate_start(&input).is_ok());
        assert!(
            validate_start(&StartGenerationRun {
                operation: " ".into(),
                ..input.clone()
            })
            .is_err()
        );
        assert!(
            validate_start(&StartGenerationRun {
                content_version: 0,
                ..input
            })
            .is_err()
        );
    }
}
