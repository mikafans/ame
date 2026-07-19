//! Application service for the first learner bootstrap.

use crate::domain::identity::{
    AccountStatus, CreateLearner, IdentityRepository, IdentityRepositoryError, LearnerAccount,
    RegistrationMode,
};
use crate::domain::learning::{
    CreateGoal, CreateJourney, LearningGoal, LearningJourney, LearningRepository,
    LearningRepositoryError,
};
use crate::templates::builtin_catalog;
use async_trait::async_trait;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartLearningRequest {
    pub authenticated_user_id: Option<Uuid>,
    pub email: String,
    pub display_name: String,
    pub raw_intent: String,
    pub normalized_statement: String,
    pub promise: String,
    pub template_id: String,
    pub template_version: u32,
    pub template_version_id: Option<Uuid>,
    pub idempotency_key: String,
    pub registration_mode: RegistrationMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartLearningResult {
    pub account: LearnerAccount,
    pub bootstrap: BootstrapResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartLearningPromptRequest {
    pub authenticated_user_id: Option<Uuid>,
    pub email: String,
    pub display_name: String,
    pub raw_prompt: String,
    pub idempotency_key: String,
    pub registration_mode: RegistrationMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptInterpretation {
    pub normalized_statement: String,
    pub promise: String,
    pub template_id: String,
    pub template_version: u32,
    pub template_version_id: Option<Uuid>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PromptInterpretationError {
    #[error("learning prompt must not be empty")]
    EmptyPrompt,
    #[error("built-in learning template catalog is invalid: {0}")]
    InvalidTemplateCatalog(String),
}

#[async_trait]
pub trait LearningPromptInterpreter: Send + Sync {
    async fn interpret(
        &self,
        raw_prompt: &str,
    ) -> Result<PromptInterpretation, PromptInterpretationError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CatalogPromptInterpreter;

#[async_trait]
impl LearningPromptInterpreter for CatalogPromptInterpreter {
    async fn interpret(
        &self,
        raw_prompt: &str,
    ) -> Result<PromptInterpretation, PromptInterpretationError> {
        let normalized_statement = raw_prompt.trim();
        if normalized_statement.is_empty() {
            return Err(PromptInterpretationError::EmptyPrompt);
        }

        let recommendation = crate::templates::recommend_builtin_template(normalized_statement);
        let catalog =
            builtin_catalog().map_err(PromptInterpretationError::InvalidTemplateCatalog)?;
        let template = catalog
            .templates
            .iter()
            .find(|template| template.id == recommendation.template_id)
            .ok_or_else(|| {
                PromptInterpretationError::InvalidTemplateCatalog(format!(
                    "recommended template {} is missing",
                    recommendation.template_id
                ))
            })?;
        let promise = match template.id.as_str() {
            "exam-prep" => "Build an exam-ready understanding through guided practice and review",
            "build-a-project" => "Learn the concepts and produce a working first project",
            _ => "Build a durable foundation through guided practice and review",
        };

        Ok(PromptInterpretation {
            normalized_statement: normalized_statement.to_string(),
            promise: promise.to_string(),
            template_id: template.id.clone(),
            template_version: template.version,
            template_version_id: None,
        })
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum StartLearningError {
    #[error("self-host onboarding only supports open registration")]
    UnsupportedRegistrationMode,
    #[error(transparent)]
    Identity(#[from] IdentityRepositoryError),
    #[error(transparent)]
    Bootstrap(#[from] BootstrapError),
    #[error(transparent)]
    Prompt(#[from] PromptInterpretationError),
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

#[derive(Clone)]
pub struct SelfHostOnboardingService<I, L> {
    identity: I,
    learning: OnboardingService<L>,
}

impl<I, L> SelfHostOnboardingService<I, L>
where
    I: IdentityRepository,
    L: LearningRepository,
{
    pub fn new(identity: I, learning: L) -> Self {
        Self {
            identity,
            learning: OnboardingService::new(learning),
        }
    }

    pub async fn start_learning(
        &self,
        request: StartLearningRequest,
    ) -> Result<StartLearningResult, StartLearningError> {
        if request.registration_mode != RegistrationMode::Open {
            return Err(StartLearningError::UnsupportedRegistrationMode);
        }

        let account = if let Some(user_id) = request.authenticated_user_id {
            let account = self.identity.get_learner(user_id).await?;
            if crate::auth::token::canonical_email(&request.email) != account.email {
                return Err(IdentityRepositoryError::AccountEmailMismatch.into());
            }
            account
        } else {
            match self.identity.find_learner_by_email(&request.email).await? {
                Some(_) => return Err(IdentityRepositoryError::AccountAlreadyExists.into()),
                None => {
                    self.identity
                        .create_learner(CreateLearner {
                            email: request.email.clone(),
                            display_name: request.display_name.clone(),
                        })
                        .await?
                }
            }
        };
        if account.status != AccountStatus::Active {
            return Err(IdentityRepositoryError::AccountNotFound.into());
        }

        let bootstrap = self
            .learning
            .bootstrap(BootstrapPlan {
                subject_user_id: account.id,
                source_actor_id: account.id,
                raw_intent: request.raw_intent,
                normalized_statement: request.normalized_statement,
                promise: request.promise,
                template_id: request.template_id,
                template_version: request.template_version,
                template_version_id: request.template_version_id,
                idempotency_key: request.idempotency_key,
            })
            .await?;

        Ok(StartLearningResult { account, bootstrap })
    }

    pub async fn start_from_prompt<P: LearningPromptInterpreter>(
        &self,
        request: StartLearningPromptRequest,
        interpreter: &P,
    ) -> Result<StartLearningResult, StartLearningError> {
        let interpretation = interpreter.interpret(&request.raw_prompt).await?;
        self.start_learning(StartLearningRequest {
            authenticated_user_id: request.authenticated_user_id,
            email: request.email,
            display_name: request.display_name,
            raw_intent: request.raw_prompt,
            normalized_statement: interpretation.normalized_statement,
            promise: interpretation.promise,
            template_id: interpretation.template_id,
            template_version: interpretation.template_version,
            template_version_id: interpretation.template_version_id,
            idempotency_key: request.idempotency_key,
            registration_mode: request.registration_mode,
        })
        .await
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
    use super::{
        BootstrapError, BootstrapPlan, CatalogPromptInterpreter, OnboardingService,
        SelfHostOnboardingService, StartLearningError, StartLearningPromptRequest,
        StartLearningRequest,
    };
    use crate::domain::identity::RegistrationMode;
    use crate::domain::learning::LearningRepositoryError;
    use crate::identity::InMemoryIdentityRepository;
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

    fn start_learning_request() -> StartLearningRequest {
        StartLearningRequest {
            authenticated_user_id: None,
            email: "learner@example.test".to_string(),
            display_name: "Learner".to_string(),
            raw_intent: "I would like to learn music theory".to_string(),
            normalized_statement: "Understand foundational music theory".to_string(),
            promise: "Identify notes, intervals, and basic chords".to_string(),
            template_id: "learn-a-subject".to_string(),
            template_version: 1,
            template_version_id: None,
            idempotency_key: "onboarding/music-theory".to_string(),
            registration_mode: RegistrationMode::Open,
        }
    }

    fn start_learning_prompt_request() -> StartLearningPromptRequest {
        StartLearningPromptRequest {
            authenticated_user_id: None,
            email: "prompt-learner@example.test".to_string(),
            display_name: "Prompt Learner".to_string(),
            raw_prompt: "I would like to learn music theory".to_string(),
            idempotency_key: "onboarding/prompt-music-theory".to_string(),
            registration_mode: RegistrationMode::Open,
        }
    }

    #[tokio::test]
    async fn start_from_prompt_uses_replaceable_interpreter_contract() {
        let service = SelfHostOnboardingService::new(
            InMemoryIdentityRepository::default(),
            InMemoryLearningRepository::default(),
        );
        let result = service
            .start_from_prompt(start_learning_prompt_request(), &CatalogPromptInterpreter)
            .await
            .expect("prompt onboarding succeeds");

        assert_eq!(result.bootstrap.template_id, "learn-a-subject");
        assert_eq!(
            result.bootstrap.goal.raw_intent,
            "I would like to learn music theory"
        );
    }

    #[tokio::test]
    async fn self_host_start_learning_requires_authentication_to_retry() {
        let service = SelfHostOnboardingService::new(
            InMemoryIdentityRepository::default(),
            InMemoryLearningRepository::default(),
        );
        let request = start_learning_request();

        let first = service
            .start_learning(request.clone())
            .await
            .expect("first learning start succeeds");
        let mut retry_request = request;
        retry_request.authenticated_user_id = Some(first.account.id);
        let retry = service
            .start_learning(retry_request)
            .await
            .expect("authenticated retry succeeds");

        assert_eq!(first, retry);
        assert_eq!(first.bootstrap.goal.subject_user_id, first.account.id);
        assert_eq!(first.bootstrap.goal.source_actor_id, first.account.id);
    }

    #[tokio::test]
    async fn self_host_start_learning_rejects_existing_email_without_authentication() {
        let service = SelfHostOnboardingService::new(
            InMemoryIdentityRepository::default(),
            InMemoryLearningRepository::default(),
        );
        let request = start_learning_request();
        service
            .start_learning(request.clone())
            .await
            .expect("first learning start succeeds");

        assert_eq!(
            service.start_learning(request).await,
            Err(StartLearningError::Identity(
                crate::domain::identity::IdentityRepositoryError::AccountAlreadyExists
            ))
        );
    }

    #[tokio::test]
    async fn self_host_start_learning_does_not_accept_password_mode_yet() {
        let service = SelfHostOnboardingService::new(
            InMemoryIdentityRepository::default(),
            InMemoryLearningRepository::default(),
        );
        let mut request = start_learning_request();
        request.registration_mode = RegistrationMode::Password;

        assert_eq!(
            service.start_learning(request).await,
            Err(StartLearningError::UnsupportedRegistrationMode)
        );
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
