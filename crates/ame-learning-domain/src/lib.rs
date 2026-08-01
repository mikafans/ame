//! Domain contracts for the first agent-first learning journey.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum GoalStatus {
    Proposed,
    Active,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum JourneyStatus {
    Onboarding,
    Active,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveStatus {
    Active,
    Paused,
    Completed,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActivityKind {
    Explanation,
    Example,
    Diagnostic,
    Practice,
    Feedback,
    Application,
    Reflection,
    Milestone,
    TimedPractice,
    Recommendation,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActivityStatus {
    Proposed,
    Ready,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActivityPublicationStatus {
    Draft,
    Review,
    Published,
    Retired,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LearningSessionStatus {
    InProgress,
    Finished,
    Abandoned,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateGoal {
    pub subject_user_id: Uuid,
    pub source_actor_id: Uuid,
    pub template_version_id: Option<Uuid>,
    pub catalog_entry_id: Option<String>,
    pub catalog_entry_version: Option<u32>,
    pub raw_intent: String,
    pub normalized_statement: String,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearningGoal {
    pub id: Uuid,
    pub subject_user_id: Uuid,
    pub source_actor_id: Uuid,
    pub template_version_id: Option<Uuid>,
    pub catalog_entry_id: Option<String>,
    pub catalog_entry_version: Option<u32>,
    pub raw_intent: String,
    pub normalized_statement: String,
    pub status: GoalStatus,
    pub idempotency_key: Option<String>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateJourney {
    pub goal_id: Uuid,
    pub subject_user_id: Uuid,
    pub source_actor_id: Uuid,
    pub promise: String,
    pub catalog_entry_id: Option<String>,
    pub catalog_entry_version: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearningJourney {
    pub id: Uuid,
    pub goal_id: Uuid,
    pub subject_user_id: Uuid,
    pub source_actor_id: Uuid,
    pub promise: String,
    pub catalog_entry_id: Option<String>,
    pub catalog_entry_version: Option<u32>,
    pub status: JourneyStatus,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateChapter {
    pub journey_id: Uuid,
    pub subject_user_id: Uuid,
    pub title: String,
    pub summary: String,
    pub order_index: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearningChapter {
    pub id: Uuid,
    pub journey_id: Uuid,
    pub subject_user_id: Uuid,
    pub title: String,
    pub summary: String,
    pub order_index: i32,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateObjective {
    pub journey_id: Uuid,
    pub subject_user_id: Uuid,
    pub verb: String,
    pub statement: String,
    pub success_criteria: String,
    pub order_index: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearningObjective {
    pub id: Uuid,
    pub journey_id: Uuid,
    pub subject_user_id: Uuid,
    pub verb: String,
    pub statement: String,
    pub success_criteria: String,
    pub order_index: i32,
    pub status: ObjectiveStatus,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateActivity {
    pub journey_id: Uuid,
    pub subject_user_id: Uuid,
    pub source_actor_id: Uuid,
    pub chapter_id: Option<Uuid>,
    pub kind: ActivityKind,
    pub title: String,
    pub order_index: i32,
    pub payload_schema_version: i32,
    pub content_version: i32,
    pub publication_status: ActivityPublicationStatus,
    pub payload: serde_json::Value,
    pub objective_ids: Vec<Uuid>,
    pub status: ActivityStatus,
    /// Optional scoring rubric for task/application activities. Opaque JSON here;
    /// the API layer validates its shape against the platform `TaskRubric` type.
    pub rubric: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LearningActivity {
    pub id: Uuid,
    pub journey_id: Uuid,
    pub subject_user_id: Uuid,
    pub source_actor_id: Uuid,
    pub chapter_id: Option<Uuid>,
    pub kind: ActivityKind,
    pub title: String,
    pub order_index: i32,
    pub payload_schema_version: i32,
    pub content_version: i32,
    pub publication_status: ActivityPublicationStatus,
    pub payload: serde_json::Value,
    pub objective_ids: Vec<Uuid>,
    pub status: ActivityStatus,
    /// Optional scoring rubric for task/application activities (opaque JSON;
    /// shape validated at the API boundary against the platform `TaskRubric`).
    pub rubric: Option<serde_json::Value>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AuthorActivityContent {
    pub subject_user_id: Uuid,
    pub activity_id: Uuid,
    pub generation_run_id: Uuid,
    pub content: serde_json::Value,
    pub source_references: Vec<String>,
    pub review_status: String,
}

/// Provenance-gated authoring of a task activity's scoring rubric. The `rubric`
/// payload is opaque here; its structured shape is validated at the API layer.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthorActivityRubric {
    pub subject_user_id: Uuid,
    pub activity_id: Uuid,
    pub generation_run_id: Uuid,
    pub rubric: serde_json::Value,
    pub source_references: Vec<String>,
    pub review_status: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActivityCapability {
    Explanation {
        heading: String,
        body: String,
        key_points: Vec<String>,
    },
    WorkedExample {
        heading: String,
        prompt: String,
        steps: Vec<String>,
        reflection: String,
    },
    RichText {
        heading: String,
        body: String,
    },
    Diagram {
        title: String,
        source: String,
        alt_text: String,
    },
    CodeExample {
        title: String,
        language: String,
        code: String,
        explanation: String,
    },
    Scenario {
        context: String,
        prompt: String,
        options: Vec<ScenarioOption>,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ScenarioOption {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateLearningSession {
    pub journey_id: Uuid,
    pub activity_id: Uuid,
    pub subject_user_id: Uuid,
    pub actor_identity_id: Uuid,
    pub question_plan: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FinishLearningSession {
    pub subject_user_id: Uuid,
    pub session_id: Uuid,
    pub completed: bool,
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LearningSession {
    pub id: Uuid,
    pub journey_id: Uuid,
    pub activity_id: Uuid,
    pub subject_user_id: Uuid,
    pub actor_identity_id: Uuid,
    pub status: LearningSessionStatus,
    pub question_plan: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub started_at: OffsetDateTime,
    pub finished_at: Option<OffsetDateTime>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LearningRepositoryError {
    #[error("learning field {field} must not be empty")]
    EmptyField { field: &'static str },
    #[error("learning resource not found: {resource}")]
    NotFound { resource: &'static str },
    #[error("learning resource belongs to another subject")]
    SubjectMismatch,
    #[error("idempotency key was reused with a different goal")]
    IdempotencyConflict,
    #[error("a journey already exists for this goal")]
    JourneyAlreadyExists,
    #[error("{resource} order already exists in this journey")]
    OrderConflict { resource: &'static str },
    #[error("activity is not ready to start")]
    ActivityNotReady,
    #[error("activity content is invalid")]
    InvalidActivityContent,
    #[error("activity rubric is invalid")]
    InvalidRubric,
    #[error("completed activity content cannot be changed")]
    ActivityContentCompleted,
    #[error("learning session is already finished")]
    LearningSessionFinished,
    #[error("learning session was finished with a different result")]
    LearningSessionResultConflict,
    #[error("learning repository storage failure: {0}")]
    Storage(String),
}

impl LearningRepositoryError {
    pub fn storage(error: impl Display) -> Self {
        Self::Storage(error.to_string())
    }
}

#[async_trait]
pub trait LearningRepository: Send + Sync {
    async fn create_goal(&self, input: CreateGoal)
    -> Result<LearningGoal, LearningRepositoryError>;

    async fn get_goal(
        &self,
        subject_user_id: Uuid,
        goal_id: Uuid,
    ) -> Result<LearningGoal, LearningRepositoryError>;

    async fn ensure_journey(
        &self,
        input: CreateJourney,
    ) -> Result<LearningJourney, LearningRepositoryError>;

    async fn get_journey(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
    ) -> Result<LearningJourney, LearningRepositoryError>;

    async fn list_journeys(
        &self,
        subject_user_id: Uuid,
    ) -> Result<Vec<LearningJourney>, LearningRepositoryError>;

    async fn set_journey_status(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
        status: JourneyStatus,
    ) -> Result<LearningJourney, LearningRepositoryError>;

    async fn create_objective(
        &self,
        input: CreateObjective,
    ) -> Result<LearningObjective, LearningRepositoryError>;

    async fn list_objectives(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
    ) -> Result<Vec<LearningObjective>, LearningRepositoryError>;

    async fn create_chapter(
        &self,
        input: CreateChapter,
    ) -> Result<LearningChapter, LearningRepositoryError>;

    async fn list_chapters(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
    ) -> Result<Vec<LearningChapter>, LearningRepositoryError>;

    async fn create_activity(
        &self,
        input: CreateActivity,
    ) -> Result<LearningActivity, LearningRepositoryError>;

    async fn list_activities(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
    ) -> Result<Vec<LearningActivity>, LearningRepositoryError>;

    async fn author_activity_content(
        &self,
        input: AuthorActivityContent,
    ) -> Result<LearningActivity, LearningRepositoryError>;

    async fn author_activity_rubric(
        &self,
        input: AuthorActivityRubric,
    ) -> Result<LearningActivity, LearningRepositoryError>;

    async fn start_learning_session(
        &self,
        input: CreateLearningSession,
    ) -> Result<LearningSession, LearningRepositoryError>;

    async fn get_learning_session(
        &self,
        subject_user_id: Uuid,
        session_id: Uuid,
    ) -> Result<LearningSession, LearningRepositoryError>;

    async fn latest_finished_learning_session(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
    ) -> Result<Option<LearningSession>, LearningRepositoryError>;

    async fn finish_learning_session(
        &self,
        input: FinishLearningSession,
    ) -> Result<LearningSession, LearningRepositoryError>;
}

pub fn validate_goal(input: &CreateGoal) -> Result<(), LearningRepositoryError> {
    if input.raw_intent.trim().is_empty() {
        return Err(LearningRepositoryError::EmptyField {
            field: "raw_intent",
        });
    }
    if input.normalized_statement.trim().is_empty() {
        return Err(LearningRepositoryError::EmptyField {
            field: "normalized_statement",
        });
    }
    if input
        .idempotency_key
        .as_ref()
        .is_some_and(|key| key.trim().is_empty())
    {
        return Err(LearningRepositoryError::EmptyField {
            field: "idempotency_key",
        });
    }
    Ok(())
}

pub fn validate_journey(input: &CreateJourney) -> Result<(), LearningRepositoryError> {
    if input.promise.trim().is_empty() {
        return Err(LearningRepositoryError::EmptyField { field: "promise" });
    }
    Ok(())
}

pub fn validate_objective(input: &CreateObjective) -> Result<(), LearningRepositoryError> {
    for (field, value) in [
        ("verb", input.verb.as_str()),
        ("statement", input.statement.as_str()),
        ("success_criteria", input.success_criteria.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(LearningRepositoryError::EmptyField { field });
        }
    }
    if input.order_index < 0 {
        return Err(LearningRepositoryError::EmptyField {
            field: "order_index",
        });
    }
    Ok(())
}

pub fn validate_chapter(input: &CreateChapter) -> Result<(), LearningRepositoryError> {
    for (field, value) in [
        ("title", input.title.as_str()),
        ("summary", input.summary.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(LearningRepositoryError::EmptyField { field });
        }
    }
    if input.order_index < 0 {
        return Err(LearningRepositoryError::EmptyField {
            field: "order_index",
        });
    }
    Ok(())
}

pub fn validate_activity(input: &CreateActivity) -> Result<(), LearningRepositoryError> {
    if input.title.trim().is_empty() {
        return Err(LearningRepositoryError::EmptyField { field: "title" });
    }
    if input.order_index < 0 {
        return Err(LearningRepositoryError::EmptyField {
            field: "order_index",
        });
    }
    if input.payload_schema_version <= 0 {
        return Err(LearningRepositoryError::EmptyField {
            field: "payload_schema_version",
        });
    }
    if input.content_version <= 0 {
        return Err(LearningRepositoryError::EmptyField {
            field: "content_version",
        });
    }
    Ok(())
}

pub fn validate_activity_content(
    activity_kind: ActivityKind,
    input: &AuthorActivityContent,
) -> Result<(), LearningRepositoryError> {
    if input.source_references.is_empty()
        || input
            .source_references
            .iter()
            .any(|reference| reference.trim().is_empty())
    {
        return Err(LearningRepositoryError::EmptyField {
            field: "source_references",
        });
    }
    if input.review_status != "approved" {
        return Err(LearningRepositoryError::EmptyField {
            field: "review_status",
        });
    }
    let Some(object) = input.content.as_object() else {
        return Err(LearningRepositoryError::InvalidActivityContent);
    };
    let Some(_content_type) = object.get("type").and_then(serde_json::Value::as_str) else {
        return Err(LearningRepositoryError::InvalidActivityContent);
    };
    let capability: ActivityCapability = serde_json::from_value(input.content.clone())
        .map_err(|_| LearningRepositoryError::InvalidActivityContent)?;
    let valid_for_kind = matches!(
        (&activity_kind, &capability),
        (
            ActivityKind::Explanation,
            ActivityCapability::Explanation { .. }
        ) | (
            ActivityKind::Explanation,
            ActivityCapability::RichText { .. }
        ) | (
            ActivityKind::Explanation,
            ActivityCapability::Diagram { .. }
        ) | (
            ActivityKind::Example,
            ActivityCapability::WorkedExample { .. }
        ) | (
            ActivityKind::Example,
            ActivityCapability::CodeExample { .. }
        ) | (ActivityKind::Practice, ActivityCapability::Scenario { .. })
            | (
                ActivityKind::Application,
                ActivityCapability::Scenario { .. }
            )
    );
    if !valid_for_kind || !capability_has_content(&capability) {
        return Err(LearningRepositoryError::InvalidActivityContent);
    }
    Ok(())
}

/// Provenance gate + shape check for an authored rubric. The rubric's structured
/// contents (criteria, points, passing score) are validated at the API boundary
/// against the platform `TaskRubric` type; here we only enforce the same
/// provenance discipline as content authoring and that the payload is an object.
pub fn validate_activity_rubric(
    input: &AuthorActivityRubric,
) -> Result<(), LearningRepositoryError> {
    if input.source_references.is_empty()
        || input
            .source_references
            .iter()
            .any(|reference| reference.trim().is_empty())
    {
        return Err(LearningRepositoryError::EmptyField {
            field: "source_references",
        });
    }
    if input.review_status != "approved" {
        return Err(LearningRepositoryError::EmptyField {
            field: "review_status",
        });
    }
    if !input.rubric.is_object() {
        return Err(LearningRepositoryError::InvalidRubric);
    }
    Ok(())
}

fn capability_has_content(capability: &ActivityCapability) -> bool {
    let non_empty = |value: &str| !value.trim().is_empty();
    match capability {
        ActivityCapability::Explanation {
            heading,
            body,
            key_points,
        } => non_empty(heading) && non_empty(body) && non_empty_list(key_points),
        ActivityCapability::WorkedExample {
            heading,
            prompt,
            steps,
            reflection,
        } => {
            non_empty(heading)
                && non_empty(prompt)
                && non_empty(reflection)
                && non_empty_list(steps)
        }
        ActivityCapability::RichText { heading, body } => non_empty(heading) && non_empty(body),
        ActivityCapability::Diagram {
            title,
            source,
            alt_text,
        } => non_empty(title) && non_empty(source) && non_empty(alt_text),
        ActivityCapability::CodeExample {
            title,
            language,
            code,
            explanation,
        } => non_empty(title) && non_empty(language) && non_empty(code) && non_empty(explanation),
        ActivityCapability::Scenario {
            context,
            prompt,
            options,
        } => {
            non_empty(context)
                && non_empty(prompt)
                && options.len() >= 2
                && options
                    .iter()
                    .all(|option| non_empty(&option.id) && non_empty(&option.label))
        }
    }
}

fn non_empty_list(values: &[String]) -> bool {
    !values.is_empty() && values.iter().all(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod capability_tests {
    use super::*;

    fn authored(content: serde_json::Value) -> AuthorActivityContent {
        AuthorActivityContent {
            subject_user_id: Uuid::now_v7(),
            activity_id: Uuid::now_v7(),
            generation_run_id: Uuid::now_v7(),
            content,
            source_references: vec!["https://example.com/flink".to_string()],
            review_status: "approved".to_string(),
        }
    }

    #[test]
    fn accepts_versioned_learning_capabilities_for_matching_activity_kinds() {
        let cases = [
            (
                ActivityKind::Explanation,
                serde_json::json!({
                    "type": "rich_text", "heading": "State", "body": "State is durable."
                }),
            ),
            (
                ActivityKind::Explanation,
                serde_json::json!({
                    "type": "diagram", "title": "Flow", "source": "graph TD; A-->B", "alt_text": "A flows to B"
                }),
            ),
            (
                ActivityKind::Example,
                serde_json::json!({
                    "type": "code_example", "title": "Job", "language": "java", "code": "env.execute();", "explanation": "Runs the job."
                }),
            ),
            (
                ActivityKind::Practice,
                serde_json::json!({
                    "type": "scenario", "context": "A task fails.", "prompt": "What do you inspect first?",
                    "options": [{"id": "logs", "label": "Inspect logs"}, {"id": "retry", "label": "Retry immediately"}]
                }),
            ),
        ];

        for (kind, content) in cases {
            assert!(validate_activity_content(kind, &authored(content)).is_ok());
        }
    }

    fn authored_rubric(rubric: serde_json::Value) -> AuthorActivityRubric {
        AuthorActivityRubric {
            subject_user_id: Uuid::now_v7(),
            activity_id: Uuid::now_v7(),
            generation_run_id: Uuid::now_v7(),
            rubric,
            source_references: vec!["https://example.com/rubric".to_string()],
            review_status: "approved".to_string(),
        }
    }

    #[test]
    fn rubric_authoring_requires_provenance_and_object_payload() {
        let ok = authored_rubric(serde_json::json!({
            "version": 1,
            "criteria": [{"id": "c1", "objectiveId": Uuid::now_v7(), "description": "d", "maxPoints": 4, "required": true}]
        }));
        assert!(validate_activity_rubric(&ok).is_ok());

        let mut no_sources = ok.clone();
        no_sources.source_references = vec![];
        assert_eq!(
            validate_activity_rubric(&no_sources),
            Err(LearningRepositoryError::EmptyField {
                field: "source_references"
            })
        );

        let mut unapproved = ok.clone();
        unapproved.review_status = "draft".to_string();
        assert_eq!(
            validate_activity_rubric(&unapproved),
            Err(LearningRepositoryError::EmptyField {
                field: "review_status"
            })
        );

        let not_object = authored_rubric(serde_json::json!([1, 2, 3]));
        assert_eq!(
            validate_activity_rubric(&not_object),
            Err(LearningRepositoryError::InvalidRubric)
        );
    }

    #[test]
    fn rejects_unknown_or_malformed_capabilities() {
        for content in [
            serde_json::json!({"type": "interactive_lab", "title": "Run it"}),
            serde_json::json!({"type": "diagram", "title": "Flow", "source": "graph TD", "alt_text": ""}),
            serde_json::json!({"type": "scenario", "context": "Context", "prompt": "Choose", "options": [{"id": "only", "label": "Only choice"}]}),
        ] {
            assert_eq!(
                validate_activity_content(ActivityKind::Explanation, &authored(content)),
                Err(LearningRepositoryError::InvalidActivityContent)
            );
        }
    }
}
