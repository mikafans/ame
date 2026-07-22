use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::admin::RequireAdmin,
    auth::extractor::AuthenticatedUser,
    domain::error::ApiError,
    domain::progress::{EvidenceSource, MasteryEvidenceInput},
    domain::task::TaskEvaluationMethod,
    http::AppState,
    progress::ProgressRepository,
    progress_postgres::PgProgressRepository,
};
use ame_platform_application::task::TaskSubmissionRepository;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StartTaskSubmissionBody {
    pub content_version: u32,
    pub response: serde_json::Value,
    #[serde(default = "default_evaluation_method")]
    pub evaluation_method: TaskEvaluationMethod,
}

fn default_evaluation_method() -> TaskEvaluationMethod {
    TaskEvaluationMethod::SelfReview
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskSubmissionResponse {
    pub id: Uuid,
    pub task_id: Uuid,
    pub journey_id: Uuid,
    pub content_version: u32,
    pub response: serde_json::Value,
    pub evaluation_method: TaskEvaluationMethod,
    pub status: crate::domain::task::TaskSubmissionStatus,
    pub review_status: crate::domain::task::TaskReviewStatus,
    pub score: Option<f32>,
    pub feedback: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReviewTaskSubmissionBody {
    pub outcome: ReviewTaskOutcome,
    pub score: Option<f32>,
    pub feedback: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReviewTaskOutcome {
    Reviewed,
    Rejected,
}

#[utoipa::path(
    post,
    path = "/api/v1/tasks/{task_id}/submissions",
    params(("task_id" = Uuid, Path, description = "Application task activity ID")),
    request_body = StartTaskSubmissionBody,
    responses((status = 201, body = TaskSubmissionResponse)),
    security(("bearer" = [])),
    tag = "tasks"
)]
pub async fn start_submission(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(task_id): Path<Uuid>,
    Json(body): Json<StartTaskSubmissionBody>,
) -> Result<(axum::http::StatusCode, Json<TaskSubmissionResponse>), ApiError> {
    let submission =
        ame_platform_postgres::task_postgres::PgTaskSubmissionRepository::new(state.pool.clone())
            .start(ame_platform_application::task::StartTaskSubmission {
                subject_user_id: auth.owner_id(),
                task_id,
                content_version: body.content_version,
                response: body.response,
                evaluation_method: body.evaluation_method,
            })
            .await
            .map_err(map_task_error)?;
    Ok((axum::http::StatusCode::CREATED, Json(response(submission))))
}

#[utoipa::path(
    post,
    path = "/api/v1/task-submissions/{submission_id}/submit",
    params(("submission_id" = Uuid, Path, description = "Task submission ID")),
    responses((status = 200, body = TaskSubmissionResponse)),
    security(("bearer" = [])),
    tag = "tasks"
)]
pub async fn submit_submission(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(submission_id): Path<Uuid>,
) -> Result<Json<TaskSubmissionResponse>, ApiError> {
    let submission =
        ame_platform_postgres::task_postgres::PgTaskSubmissionRepository::new(state.pool.clone())
            .submit(auth.owner_id(), submission_id)
            .await
            .map_err(map_task_error)?;
    Ok(Json(response(submission)))
}

fn response(submission: ame_platform_application::task::TaskSubmission) -> TaskSubmissionResponse {
    TaskSubmissionResponse {
        id: submission.id,
        task_id: submission.envelope.task_id,
        journey_id: submission.journey_id,
        content_version: submission.envelope.content_version,
        response: submission.envelope.response,
        evaluation_method: submission.envelope.evaluation_method,
        status: submission.envelope.status,
        review_status: submission.envelope.review_status,
        score: submission.envelope.score,
        feedback: submission.envelope.feedback,
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/task-submissions/{submission_id}",
    params(("submission_id" = Uuid, Path, description = "Task submission ID")),
    responses((status = 200, body = TaskSubmissionResponse)),
    security(("bearer" = [])),
    tag = "tasks"
)]
pub async fn get_submission(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(submission_id): Path<Uuid>,
) -> Result<Json<TaskSubmissionResponse>, ApiError> {
    let submission =
        ame_platform_postgres::task_postgres::PgTaskSubmissionRepository::new(state.pool.clone())
            .get_for_subject(auth.owner_id(), submission_id)
            .await
            .map_err(map_task_error)?;
    Ok(Json(response(submission)))
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/task-submissions",
    responses((status = 200, body = [TaskSubmissionResponse])),
    security(("bearer" = [])),
    tag = "tasks"
)]
pub async fn list_pending_submissions(
    State(state): State<AppState>,
    _admin: RequireAdmin,
) -> Result<Json<Vec<TaskSubmissionResponse>>, ApiError> {
    let submissions =
        ame_platform_postgres::task_postgres::PgTaskSubmissionRepository::new(state.pool.clone())
            .list_pending_for_admin()
            .await
            .map_err(map_task_error)?;
    Ok(Json(submissions.into_iter().map(response).collect()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/task-submissions/{submission_id}/review",
    params(("submission_id" = Uuid, Path, description = "Task submission ID")),
    request_body = ReviewTaskSubmissionBody,
    responses((status = 200, body = TaskSubmissionResponse)),
    security(("bearer" = [])),
    tag = "tasks"
)]
pub async fn review_submission(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(submission_id): Path<Uuid>,
    Json(body): Json<ReviewTaskSubmissionBody>,
) -> Result<Json<TaskSubmissionResponse>, ApiError> {
    let is_reviewed = matches!(body.outcome, ReviewTaskOutcome::Reviewed);
    let outcome = match body.outcome {
        ReviewTaskOutcome::Reviewed => ame_platform_application::task::TaskReviewOutcome::Reviewed,
        ReviewTaskOutcome::Rejected => ame_platform_application::task::TaskReviewOutcome::Rejected,
    };
    let task_repository =
        ame_platform_postgres::task_postgres::PgTaskSubmissionRepository::new(state.pool.clone());
    let submission = task_repository
        .review(ame_platform_application::task::ReviewTaskSubmission {
            reviewer_user_id: _admin.0.user.id,
            submission_id,
            outcome,
            score: body.score,
            feedback: body.feedback,
        })
        .await
        .map_err(map_task_error)?;
    if is_reviewed && submission.envelope.score.is_some() {
        let context = task_repository
            .evidence_context(submission.id)
            .await
            .map_err(map_task_error)?;
        let progress = PgProgressRepository::new(state.pool.clone());
        for objective_id in context.objective_ids {
            progress
                .record_evidence(MasteryEvidenceInput {
                    subject_user_id: context.subject_user_id,
                    journey_id: context.journey_id,
                    objective_id,
                    activity_id: context.activity_id,
                    source: EvidenceSource::Task {
                        submission_id: submission.id,
                    },
                    content_version: context.content_version,
                    value: context.score,
                    derivation_version: 1,
                })
                .await
                .map_err(|error| ApiError::Internal(anyhow::anyhow!(error.to_string())))?;
        }
    }
    Ok(Json(response(submission)))
}

fn map_task_error(error: ame_platform_application::task::TaskSubmissionError) -> ApiError {
    match error {
        ame_platform_application::task::TaskSubmissionError::NotFound
        | ame_platform_application::task::TaskSubmissionError::SubjectMismatch
        | ame_platform_application::task::TaskSubmissionError::StaleContentVersion => {
            ApiError::NotFound { resource: "task" }
        }
        ame_platform_application::task::TaskSubmissionError::InvalidContract(message) => {
            ApiError::Validation(vec![crate::domain::error::FieldError {
                field: "submission".into(),
                message,
            }])
        }
        ame_platform_application::task::TaskSubmissionError::InvalidTransition(_error) => {
            ApiError::IdempotencyConflict
        }
        ame_platform_application::task::TaskSubmissionError::Storage(message) => {
            ApiError::Internal(anyhow::anyhow!(message))
        }
    }
}
