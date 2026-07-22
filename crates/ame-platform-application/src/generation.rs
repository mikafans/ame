//! In-memory generation-run repository used by contract tests and local services.

use async_trait::async_trait;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
use time::OffsetDateTime;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::generation::{
    GenerationError, GenerationRepository, GenerationRun, GenerationStatus, StartGenerationRun,
    transition_status, validate_start,
};

#[derive(Clone, Default)]
pub struct InMemoryGenerationRepository {
    runs: Arc<RwLock<HashMap<Uuid, GenerationRun>>>,
}

#[async_trait]
impl GenerationRepository for InMemoryGenerationRepository {
    async fn start(&self, input: StartGenerationRun) -> Result<GenerationRun, GenerationError> {
        validate_start(&input)?;
        let mut runs = self.runs.write().await;
        if let Some(retry_key) = input.retry_key.as_deref()
            && let Some(existing) = runs.values().find(|run| {
                run.subject_user_id == input.subject_user_id
                    && run.operation == input.operation
                    && run.retry_key.as_deref() == Some(retry_key)
            })
        {
            return Ok(existing.clone());
        }
        let now = OffsetDateTime::now_utc();
        let run = GenerationRun {
            id: Uuid::now_v7(),
            subject_user_id: input.subject_user_id,
            source_actor_id: input.source_actor_id,
            operation: input.operation,
            provider: input.provider,
            retry_key: input.retry_key,
            content_version: input.content_version,
            status: GenerationStatus::Requested,
            error: None,
            created_at: now,
            updated_at: now,
        };
        runs.insert(run.id, run.clone());
        Ok(run)
    }

    async fn transition(
        &self,
        subject_user_id: Uuid,
        id: Uuid,
        status: GenerationStatus,
        error: Option<Value>,
    ) -> Result<GenerationRun, GenerationError> {
        let mut runs = self.runs.write().await;
        let run = runs.get_mut(&id).ok_or(GenerationError::NotFound)?;
        if run.subject_user_id != subject_user_id {
            return Err(GenerationError::SubjectMismatch);
        }
        transition_status(run.status, status)?;
        if status == GenerationStatus::Failed && error.is_none() {
            return Err(GenerationError::EmptyField { field: "error" });
        }
        if status != GenerationStatus::Failed && error.is_some() {
            return Err(GenerationError::InvalidTransition {
                from: run.status,
                to: status,
            });
        }
        run.status = status;
        run.error = error;
        run.updated_at = OffsetDateTime::now_utc();
        Ok(run.clone())
    }

    async fn get(&self, subject_user_id: Uuid, id: Uuid) -> Result<GenerationRun, GenerationError> {
        let runs = self.runs.read().await;
        let run = runs.get(&id).ok_or(GenerationError::NotFound)?;
        if run.subject_user_id != subject_user_id {
            return Err(GenerationError::SubjectMismatch);
        }
        Ok(run.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn retry_key_returns_the_same_run_and_failure_requires_error() {
        let repository = InMemoryGenerationRepository::default();
        let subject = Uuid::now_v7();
        let input = StartGenerationRun {
            subject_user_id: subject,
            source_actor_id: subject,
            operation: "question.compose".into(),
            provider: Some("external-agent".into()),
            retry_key: Some("retry-1".into()),
            content_version: 1,
        };
        let first = repository.start(input.clone()).await.expect("start run");
        let retry = repository.start(input).await.expect("retry run");
        assert_eq!(first.id, retry.id);
        assert_eq!(first.status, GenerationStatus::Requested);

        assert_eq!(
            repository
                .transition(subject, first.id, GenerationStatus::Running, None)
                .await
                .expect("run")
                .status,
            GenerationStatus::Running
        );
        assert_eq!(
            repository
                .transition(
                    subject,
                    first.id,
                    GenerationStatus::Failed,
                    Some(serde_json::json!({"code": "provider_unavailable"})),
                )
                .await
                .expect("fail")
                .status,
            GenerationStatus::Failed
        );
        assert!(
            repository
                .transition(subject, first.id, GenerationStatus::Published, None)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn generation_reads_are_owner_scoped() {
        let repository = InMemoryGenerationRepository::default();
        let subject = Uuid::now_v7();
        let run = repository
            .start(StartGenerationRun {
                subject_user_id: subject,
                source_actor_id: subject,
                operation: "deep_dive.create".into(),
                provider: None,
                retry_key: None,
                content_version: 1,
            })
            .await
            .expect("start run");
        assert_eq!(
            repository
                .get(Uuid::now_v7(), run.id)
                .await
                .expect_err("cross-owner read"),
            GenerationError::SubjectMismatch
        );
    }
}
