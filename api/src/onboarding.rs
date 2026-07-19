//! Application service for the first learner bootstrap.

use crate::domain::learning::{
    CreateGoal, CreateJourney, LearningGoal, LearningJourney, LearningRepository,
    LearningRepositoryError,
};
use crate::templates::builtin_catalog;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapPlan {
    pub subject_user_id: Uuid,
    pub source_actor_id: Uuid,
    pub raw_intent: String,
    pub normalized_statement: String,
    pub promise: String,
    pub template_id: String,
    pub template_version: u32,
    pub template_version_id: Option<Uuid>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapResult {
    pub goal: LearningGoal,
    pub journey: LearningJourney,
    pub template_id: String,
    pub template_version: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BootstrapError {
    #[error("bootstrap field {field} must not be empty")]
    EmptyField { field: &'static str },
    #[error("unknown built-in learning template: {template_id} v{version}")]
    UnknownTemplate { template_id: String, version: u32 },
    #[error("built-in learning template catalog is invalid: {0}")]
    InvalidTemplateCatalog(String),
    #[error(transparent)]
    Repository(#[from] LearningRepositoryError),
}

#[derive(Clone)]
pub struct OnboardingService<R> {
    repository: R,
}

impl<R> OnboardingService<R>
where
    R: LearningRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn bootstrap(&self, plan: BootstrapPlan) -> Result<BootstrapResult, BootstrapError> {
        validate_plan(&plan)?;
        validate_template(&plan)?;

        let goal = self
            .repository
            .create_goal(CreateGoal {
                subject_user_id: plan.subject_user_id,
                source_actor_id: plan.source_actor_id,
                template_version_id: plan.template_version_id,
                raw_intent: plan.raw_intent,
                normalized_statement: plan.normalized_statement,
                idempotency_key: Some(plan.idempotency_key),
            })
            .await?;
        let journey = self
            .repository
            .ensure_journey(CreateJourney {
                goal_id: goal.id,
                subject_user_id: goal.subject_user_id,
                source_actor_id: goal.source_actor_id,
                promise: plan.promise,
            })
            .await?;

        Ok(BootstrapResult {
            goal,
            journey,
            template_id: plan.template_id,
            template_version: plan.template_version,
        })
    }
}

fn validate_plan(plan: &BootstrapPlan) -> Result<(), BootstrapError> {
    for (field, value) in [
        ("raw_intent", plan.raw_intent.as_str()),
        ("normalized_statement", plan.normalized_statement.as_str()),
        ("promise", plan.promise.as_str()),
        ("template_id", plan.template_id.as_str()),
        ("idempotency_key", plan.idempotency_key.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(BootstrapError::EmptyField { field });
        }
    }
    Ok(())
}

fn validate_template(plan: &BootstrapPlan) -> Result<(), BootstrapError> {
    let catalog = builtin_catalog().map_err(BootstrapError::InvalidTemplateCatalog)?;
    if catalog.templates.iter().any(|template| {
        template.id == plan.template_id && template.version == plan.template_version
    }) {
        Ok(())
    } else {
        Err(BootstrapError::UnknownTemplate {
            template_id: plan.template_id.clone(),
            version: plan.template_version,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{BootstrapError, BootstrapPlan, OnboardingService};
    use crate::domain::learning::LearningRepositoryError;
    use crate::learning::InMemoryLearningRepository;
    use uuid::Uuid;

    fn music_theory_plan() -> BootstrapPlan {
        BootstrapPlan {
            subject_user_id: Uuid::now_v7(),
            source_actor_id: Uuid::now_v7(),
            raw_intent: "I would like to learn music theory".to_string(),
            normalized_statement: "Understand foundational music theory".to_string(),
            promise: "Identify notes, intervals, and basic chords".to_string(),
            template_id: "learn-a-subject".to_string(),
            template_version: 1,
            template_version_id: None,
            idempotency_key: "onboarding/music-theory".to_string(),
        }
    }

    #[tokio::test]
    async fn bootstrap_creates_and_retries_the_same_journey() {
        let service = OnboardingService::new(InMemoryLearningRepository::default());
        let plan = music_theory_plan();

        let first = service
            .bootstrap(plan.clone())
            .await
            .expect("bootstrap creates");
        let retry = service.bootstrap(plan).await.expect("bootstrap retries");

        assert_eq!(first, retry);
        assert_eq!(first.template_id, "learn-a-subject");
        assert_eq!(first.template_version, 1);
    }

    #[tokio::test]
    async fn bootstrap_rejects_unknown_template_before_writing() {
        let service = OnboardingService::new(InMemoryLearningRepository::default());
        let mut plan = music_theory_plan();
        plan.template_id = "invented-template".to_string();

        assert_eq!(
            service.bootstrap(plan).await,
            Err(BootstrapError::UnknownTemplate {
                template_id: "invented-template".to_string(),
                version: 1,
            })
        );
    }

    #[tokio::test]
    async fn bootstrap_rejects_empty_idempotency_key() {
        let service = OnboardingService::new(InMemoryLearningRepository::default());
        let mut plan = music_theory_plan();
        plan.idempotency_key = " ".to_string();

        assert_eq!(
            service.bootstrap(plan).await,
            Err(BootstrapError::EmptyField {
                field: "idempotency_key"
            })
        );
    }

    #[test]
    fn repository_error_remains_distinguishable() {
        let error = BootstrapError::Repository(LearningRepositoryError::SubjectMismatch);
        assert!(error.to_string().contains("another subject"));
    }
}
