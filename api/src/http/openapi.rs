//! OpenAPI 3 spec generation for the ame API.
//!
//! Each handler annotates its inputs/outputs with `#[utoipa::path]`; this
//! module aggregates the annotated paths and the `ToSchema`-derived types
//! into a single [`ApiDoc`].

use axum::{Router, routing::get};
use utoipa::OpenApi;

use crate::{
    domain::{
        assessment::{
            Assessment, AssessmentMode, AssessmentStatus, AssessmentVisibility,
            CreateAssessmentRequest, UpdateAssessmentRequest,
        },
        attempt::{Attempt, AttemptPresentation, AttemptResponse},
        error::{ApiError, FieldError},
        question::{Question, QuestionKind, QuestionStatus, Tag},
        session::{Session, SessionKind, SessionStatus},
        user::{Role, Scope, User},
    },
    engine::{
        graders::GradeOutcome, 
        sessions::SessionResult, 
        planner::StudyPlan, 
        elo::{QuestionRating, UserTagRating}
    },
    http::{
        AppState,
        admin::{AuditLogEntry, ListAuditLogsResponse, ListUsersResponse, PatchUserAdminBody, ModerateBody},
        agents::{ActivityEntry, ActivityResponse as AgentActivityResponse, RunResponse},
        assessments::{
            AddAssessmentQuestionBody, AddAssessmentQuestionResponse, CountAssessmentsResponse,
            GenerateAssessmentResponse, ListAssessmentsResponse,
            AssessmentDetail, AssessmentSummary, AssessmentQuestion, AssessmentSectionDetail,
        },
        auth::{LoginBody, AuthResponse, RegisterBody, UserInfo},
        me::{AgentSummary, CreateAgentBody, MeResponse, UpdateAgentBody, ListAttemptsResponse, CohortStatsResponse},
        plans::{CreatePlanBody},
        quota::Plan as QuotaPlan,
        sessions::{
            AnswerSessionBody, AnswerSessionResponse, CreateSessionBody, CreateSessionResponse,
            FinishSessionResponse, GetSessionQuestion, GetSessionResponse, ListMySessionsResponse,
            PatchSessionBody, PendingAttemptRow, SessionSummary,
        },
        stats::{DistributionBucket, ItemStats, QuizStatsParams, QuizStatsResponse},
    },
};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::http::auth::register,
        crate::http::auth::login,
        crate::http::auth::logout,
        crate::http::me::get_me,
        crate::http::me::list_agents,
        crate::http::me::create_agent,
        crate::http::me::update_agent,
        crate::http::me::delete_agent,
        crate::http::me::get_cohort_stats,
        crate::http::me::list_attempts,
        crate::http::assessments::list_assessments,
        crate::http::assessments::create_assessment,
        crate::http::assessments::get_assessment,
        crate::http::assessments::patch_assessment,
        crate::http::assessments::delete_assessment,
        crate::http::assessments::add_assessment_question,
        crate::http::assessments::remove_assessment_question,
        crate::http::assessments::count_assessments,
        crate::http::assessments::generate_assessment,
        crate::http::assessments::explore,
        crate::http::sessions::list_my_sessions,
        crate::http::sessions::create_session,
        crate::http::sessions::get_session,
        crate::http::sessions::patch_session,
        crate::http::sessions::answer,
        crate::http::sessions::finish,
        crate::http::sessions::list_pending_attempts,
        crate::http::sessions::grade_attempt,
        crate::http::plans::create_plan,
        crate::http::plans::get_plan,
        crate::http::tags::list_tags,
        crate::http::admin::list_users,
        crate::http::admin::patch_user_admin,
        crate::http::admin::list_audit_logs,
        crate::http::admin::moderate_quiz,
    ),
    components(
        schemas(
            User, Role, Scope, Tag, Question, QuestionKind, QuestionStatus,
            Assessment, AssessmentDetail, AssessmentSummary, AssessmentQuestion,
            AssessmentSectionDetail, AssessmentMode, AssessmentStatus, AssessmentVisibility,
            CreateAssessmentRequest, UpdateAssessmentRequest,
            AddAssessmentQuestionBody, AddAssessmentQuestionResponse,
            ListAssessmentsResponse, CountAssessmentsResponse, GenerateAssessmentResponse,
            Attempt, AttemptResponse, AttemptPresentation, ApiError, FieldError,
            Session, SessionKind, SessionStatus, QuestionRating, UserTagRating,
            LoginBody, AuthResponse, RegisterBody, UserInfo, MeResponse, UpdateAgentBody,
            AgentSummary, CreateAgentBody, CreateSessionBody, CreateSessionResponse,
            GetSessionResponse, GetSessionQuestion, AnswerSessionBody, AnswerSessionResponse,
            FinishSessionResponse, ListMySessionsResponse, SessionSummary,
            PatchSessionBody, GradeOutcome, SessionResult, StudyPlan, CreatePlanBody,
            ListUsersResponse, PatchUserAdminBody,
            ListAuditLogsResponse, AuditLogEntry, ModerateBody, AgentActivityResponse, ActivityEntry,
            RunResponse, PendingAttemptRow, DistributionBucket, ItemStats, 
            QuizStatsParams, QuizStatsResponse, 
            ListAttemptsResponse, CohortStatsResponse, QuotaPlan
        )
    ),
    info(
        title = "ame API",
        version = "0.1.0",
        description = "Adaptive Mastery Engine — quiz & exam platform with agent identity."
    )
)]
pub struct ApiDoc;

pub fn openapi_yaml() -> String {
    use utoipa::OpenApi;
    ApiDoc::openapi().to_yaml().unwrap()
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/openapi.yaml", get(|| async { openapi_yaml() }))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn openapi_spec_is_current() {
        let generated = openapi_yaml();
        let committed = fs::read_to_string("openapi.yaml").expect("failed to read openapi.yaml");
        if generated.trim() != committed.trim() {
            eprintln!("--- generated ---\n{generated}\n--- committed ---\n{committed}");
            panic!("openapi.yaml is out of date with the source. Regenerate via `make openapi`.");
        }
    }
}
