//! Application service for the first learner bootstrap.

use crate::domain::identity::{
    AccountStatus, CreateLearner, IdentityRepository, IdentityRepositoryError, LearnerAccount,
    RegistrationMode,
};
use crate::domain::learning::{
    ActivityKind, ActivityStatus, CreateActivity, CreateGoal, CreateJourney, CreateObjective,
    LearningGoal, LearningJourney, LearningRepository, LearningRepositoryError,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearningPreview {
    pub interpretation: PromptInterpretation,
    pub objectives: Vec<PreviewObjective>,
    pub first_activity: PreviewActivity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewObjective {
    pub verb: String,
    pub statement: String,
    pub success_criteria: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewActivity {
    pub kind: ActivityKind,
    pub title: String,
    pub purpose: String,
    pub estimated_minutes: i64,
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
                raw_intent: plan.raw_intent.clone(),
                normalized_statement: plan.normalized_statement.clone(),
                idempotency_key: Some(plan.idempotency_key.clone()),
            })
            .await?;
        let journey = self
            .repository
            .ensure_journey(CreateJourney {
                goal_id: goal.id,
                subject_user_id: goal.subject_user_id,
                source_actor_id: goal.source_actor_id,
                promise: plan.promise.clone(),
            })
            .await?;

        let blueprint = starter_blueprint(&plan, journey.id);
        let mut objectives = self
            .repository
            .list_objectives(journey.subject_user_id, journey.id)
            .await?;
        for objective in blueprint.objectives {
            if !objectives
                .iter()
                .any(|existing| existing.order_index == objective.order_index)
            {
                self.repository.create_objective(objective).await?;
                objectives = self
                    .repository
                    .list_objectives(journey.subject_user_id, journey.id)
                    .await?;
            }
        }

        let mut activities = self
            .repository
            .list_activities(journey.subject_user_id, journey.id)
            .await?;
        for activity in blueprint.activities {
            if activities
                .iter()
                .any(|existing| existing.order_index == activity.order_index)
            {
                continue;
            }
            let objective_ids = activity
                .objective_orders
                .iter()
                .map(|order_index| {
                    objectives
                        .iter()
                        .find(|objective| objective.order_index == *order_index)
                        .map(|objective| objective.id)
                        .ok_or(LearningRepositoryError::NotFound {
                            resource: "objective",
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
            self.repository
                .create_activity(CreateActivity {
                    journey_id: journey.id,
                    subject_user_id: journey.subject_user_id,
                    source_actor_id: plan.source_actor_id,
                    source_run_id: None,
                    kind: activity.kind,
                    title: activity.title,
                    order_index: activity.order_index,
                    payload_schema_version: 1,
                    payload: activity.payload,
                    objective_ids,
                    status: activity.status,
                })
                .await?;
            activities = self
                .repository
                .list_activities(journey.subject_user_id, journey.id)
                .await?;
        }

        Ok(BootstrapResult {
            goal,
            journey,
            template_id: plan.template_id,
            template_version: plan.template_version,
        })
    }
}

struct StarterBlueprint {
    objectives: Vec<CreateObjective>,
    activities: Vec<StarterActivity>,
}

struct StarterActivity {
    kind: ActivityKind,
    title: String,
    order_index: i32,
    objective_orders: Vec<i32>,
    payload: serde_json::Value,
    status: ActivityStatus,
}

fn starter_blueprint(plan: &BootstrapPlan, journey_id: Uuid) -> StarterBlueprint {
    let subject = plan.subject_user_id;
    let objectives = match plan.template_id.as_str() {
        "exam-prep" => vec![
            (
                "map",
                "Map the target syllabus",
                "Name the main domains and their expected weight",
            ),
            (
                "solve",
                "Solve representative problems",
                "Complete a bounded set with explained reasoning",
            ),
            (
                "review",
                "Review the weakest domain",
                "Choose one evidence-backed review target",
            ),
        ],
        "build-a-project" => vec![
            (
                "define",
                "Define the first artifact",
                "Describe a small usable result and its constraints",
            ),
            (
                "apply",
                "Apply the core concept",
                "Use the concept in a concrete implementation step",
            ),
            (
                "ship",
                "Complete the first milestone",
                "Produce and inspect a small working increment",
            ),
        ],
        _ => vec![
            (
                "identify",
                "Identify the core concepts",
                "Name the essential ideas and how they relate",
            ),
            (
                "explain",
                "Explain one concept clearly",
                "Give an accurate explanation with a useful example",
            ),
            (
                "apply",
                "Apply the ideas in practice",
                "Use the ideas in a new but bounded situation",
            ),
        ],
    };
    let objectives = objectives
        .into_iter()
        .enumerate()
        .map(
            |(order_index, (verb, statement, success_criteria))| CreateObjective {
                journey_id,
                subject_user_id: subject,
                verb: verb.to_string(),
                statement: format!("{statement}: {}", plan.normalized_statement),
                success_criteria: success_criteria.to_string(),
                order_index: order_index as i32,
            },
        )
        .collect();
    let first_title = match plan.template_id.as_str() {
        "exam-prep" => "Orient on the target and estimate your starting point",
        "build-a-project" => "Define the smallest useful first milestone",
        _ => "Get oriented and see what you already know",
    };
    let second_kind = if plan.template_id == "build-a-project" {
        ActivityKind::Application
    } else {
        ActivityKind::Diagnostic
    };
    StarterBlueprint {
        objectives,
        activities: vec![
            StarterActivity {
                kind: ActivityKind::Explanation,
                title: first_title.to_string(),
                order_index: 0,
                objective_orders: vec![0],
                payload: serde_json::json!({
                    "purpose": "orientation",
                    "intent": plan.normalized_statement,
                    "estimated_minutes": 5
                }),
                status: ActivityStatus::Ready,
            },
            StarterActivity {
                kind: second_kind,
                title: "Try a short first task".to_string(),
                order_index: 1,
                objective_orders: vec![0, 1],
                payload: serde_json::json!({
                    "purpose": "diagnose_starting_point",
                    "estimated_minutes": 10
                }),
                status: ActivityStatus::Proposed,
            },
            StarterActivity {
                kind: ActivityKind::Recommendation,
                title: "Review your result and choose the next step".to_string(),
                order_index: 2,
                objective_orders: vec![2],
                payload: serde_json::json!({
                    "purpose": "next_action",
                    "estimated_minutes": 5
                }),
                status: ActivityStatus::Proposed,
            },
        ],
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

    pub async fn preview_from_prompt<P: LearningPromptInterpreter>(
        &self,
        raw_prompt: &str,
        interpreter: &P,
    ) -> Result<LearningPreview, PromptInterpretationError> {
        let interpretation = interpreter.interpret(raw_prompt).await?;
        let blueprint = starter_blueprint(
            &BootstrapPlan {
                subject_user_id: Uuid::nil(),
                source_actor_id: Uuid::nil(),
                raw_intent: interpretation.normalized_statement.clone(),
                normalized_statement: interpretation.normalized_statement.clone(),
                promise: interpretation.promise.clone(),
                template_id: interpretation.template_id.clone(),
                template_version: interpretation.template_version,
                template_version_id: interpretation.template_version_id,
                idempotency_key: "preview".to_string(),
            },
            Uuid::nil(),
        );
        let first_activity = blueprint
            .activities
            .first()
            .expect("starter blueprint must contain a first activity");
        let estimated_minutes = first_activity
            .payload
            .get("estimated_minutes")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0);
        let purpose = first_activity
            .payload
            .get("purpose")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("first learning step")
            .to_string();

        Ok(LearningPreview {
            interpretation,
            objectives: blueprint
                .objectives
                .into_iter()
                .map(|objective| PreviewObjective {
                    verb: objective.verb,
                    statement: objective.statement,
                    success_criteria: objective.success_criteria,
                })
                .collect(),
            first_activity: PreviewActivity {
                kind: first_activity.kind,
                title: first_activity.title.clone(),
                purpose,
                estimated_minutes,
            },
        })
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
    use crate::domain::learning::{ActivityKind, LearningRepository, LearningRepositoryError};
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
    async fn preview_from_prompt_does_not_require_storage_and_exposes_first_step() {
        let service = SelfHostOnboardingService::new(
            InMemoryIdentityRepository::default(),
            InMemoryLearningRepository::default(),
        );
        let preview = service
            .preview_from_prompt(
                "I would like to learn music theory",
                &CatalogPromptInterpreter,
            )
            .await
            .expect("preview succeeds");

        assert_eq!(preview.interpretation.template_id, "learn-a-subject");
        assert_eq!(preview.objectives.len(), 3);
        assert_eq!(preview.first_activity.kind, ActivityKind::Explanation);
        assert_eq!(preview.first_activity.estimated_minutes, 5);
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
        let repository = InMemoryLearningRepository::default();
        let service = OnboardingService::new(repository.clone());
        let plan = music_theory_plan();

        let first = service
            .bootstrap(plan.clone())
            .await
            .expect("bootstrap creates");
        let retry = service.bootstrap(plan).await.expect("bootstrap retries");

        assert_eq!(first, retry);
        assert_eq!(first.template_id, "learn-a-subject");
        assert_eq!(first.template_version, 1);
        let objectives = repository
            .list_objectives(first.goal.subject_user_id, first.journey.id)
            .await
            .expect("starter objectives exist");
        let activities = repository
            .list_activities(first.goal.subject_user_id, first.journey.id)
            .await
            .expect("starter activities exist");
        assert_eq!(objectives.len(), 3);
        assert_eq!(activities.len(), 3);
        assert_eq!(
            activities[0].status,
            crate::domain::learning::ActivityStatus::Ready
        );
        assert!(
            activities
                .iter()
                .all(|activity| !activity.objective_ids.is_empty())
        );
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
