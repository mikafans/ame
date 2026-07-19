//! OpenAPI 3 spec generation for the ame API.
//!
//! Each handler annotates its inputs/outputs with `#[utoipa::path]`; this
//! module aggregates the annotated paths and the `ToSchema`-derived types
//! into a single [`ApiDoc`].

use axum::{Router, routing::get};
use utoipa::OpenApi;

use crate::{
    bank::questions::{QuestionInsert, QuestionPatch},
    domain::{
        assessment::{
            Assessment, AssessmentMode, AssessmentStatus, CreateAssessmentRequest,
            UpdateAssessmentRequest,
        },
        attempt::{Attempt, AttemptPresentation, AttemptResponse},
        error::{ApiError, FieldError},
        question::{Question, QuestionKind, QuestionStatus, QuestionVersion, Tag},
        session::{Session, SessionKind, SessionStatus},
        user::{Role, Scope, User},
    },
    engine::{
        elo::{QuestionRating, UserTagRating},
        graders::GradeOutcome,
        planner::StudyPlan,
        sessions::SessionResult,
    },
    http::{
        AppState,
        admin::{
            AdminAssessmentDetail, AdminAssessmentEntry, AdminHealthResponse,
            AdminListAssessmentsResponse, AdminPreviewQuestion, AuditLogEntry,
            ListAuditLogsResponse, ListTokensResponse, ListUsersResponse, PatchUserAdminBody,
            TokenEntry, UpdateSettingsBody,
        },
        agents::{ActivityEntry, ActivityResponse as AgentActivityResponse, RunResponse},
        assessments::{
            AddAssessmentQuestionBody, AddAssessmentQuestionResponse, AssessmentDetail,
            AssessmentQuestion, AssessmentSectionDetail, AssessmentSummary,
            CountAssessmentsResponse, CreateSectionBody, GenerateAssessmentResponse,
            ListAssessmentsResponse, UpdateSectionBody,
        },
        auth::{AuthResponse, LoginBody, RegisterBody, UserInfo},
        deep_dives::{
            CreateDeepDiveBody, DeepDive, DeepDiveRevision, DeepDiveRevisionsResponse,
            ListDeepDivesQuery, ListDeepDivesResponse, PatchDeepDiveBody, PublishDeepDiveBody,
        },
        me::{
            AgentSummary, AgentTokenSummary, CohortStatsResponse, CreateAgentBody,
            CreateAgentTokenBody, CreateKeyResponse, ListAttemptsResponse, MeResponse,
            UpdateAgentBody,
        },
        onboarding::{StartLearningBody, StartLearningResponse},
        plans::CreatePlanBody,
        questions::{
            CreateQuestionsBody, CreateQuestionsResponse, ListQuestionsResponse,
            QuestionDeepenResponse,
        },
        sessions::{
            AnswerSessionBody, AnswerSessionResponse, CreateSessionBody, CreateSessionResponse,
            FinishSessionResponse, GetSessionQuestion, GetSessionResponse, ListMySessionsResponse,
            PatchSessionBody, PendingAttemptRow, SessionSummary,
        },
        stats::{AssessmentStatsParams, AssessmentStatsResponse, DistributionBucket, ItemStats},
    },
};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::http::auth::register,
        crate::http::auth::login,
        crate::http::auth::logout,
        crate::http::onboarding::start_learning,
        crate::http::me::get_me,
        crate::http::me::list_agents,
        crate::http::me::create_agent,
        crate::http::me::update_agent,
        crate::http::me::delete_agent,
        crate::http::me::create_agent_token,
        crate::http::me::revoke_agent_token,
        crate::http::me::get_cohort_stats,
        crate::http::me::list_attempts,
        crate::http::assessments::list_assessments,
        crate::http::assessments::create_assessment,
        crate::http::assessments::get_assessment,
        crate::http::assessments::patch_assessment,
        crate::http::assessments::delete_assessment,
        crate::http::assessments::add_assessment_question,
        crate::http::assessments::remove_assessment_question,
        crate::http::assessments::create_assessment_section,
        crate::http::assessments::patch_assessment_section,
        crate::http::assessments::delete_assessment_section,
        crate::http::assessments::count_assessments,
        crate::http::assessments::generate_assessment,
        crate::http::explore::explore,
        crate::http::explore::explore_facets,
        crate::http::questions::list_questions,
        crate::http::questions::get_question,
        crate::http::questions::get_question_deepen,
        crate::http::questions::list_versions,
        crate::http::questions::create_questions,
        crate::http::questions::update_question,
        crate::http::questions::promote_question,
        crate::http::questions::archive_question,
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
        crate::http::admin::get_admin_health,
        crate::http::admin::list_assessments_admin,
        crate::http::admin::get_assessment_admin,
        crate::http::admin::delete_assessment_admin,
        crate::http::admin::restore_assessment_admin,
        crate::http::admin::list_tokens,
        crate::http::admin::delete_token_admin,
        crate::http::admin::get_settings,
        crate::http::admin::put_settings,
        crate::http::deep_dives::create_deep_dive,
        crate::http::deep_dives::list_deep_dives,
        crate::http::deep_dives::get_deep_dive,
        crate::http::deep_dives::patch_deep_dive,
        crate::http::deep_dives::publish_deep_dive,
        crate::http::deep_dives::export_deep_dives,
        crate::http::deep_dives::get_deep_dive_revisions,
    ),
    components(schemas(
        User,
        Role,
        Scope,
        Tag,
        Question,
        QuestionKind,
        QuestionStatus,
        QuestionVersion,
        QuestionInsert,
        QuestionPatch,
        ListQuestionsResponse,
        QuestionDeepenResponse,
        CreateQuestionsBody,
        CreateQuestionsResponse,
        Assessment,
        AssessmentDetail,
        AssessmentSummary,
        AssessmentQuestion,
        AssessmentSectionDetail,
        AssessmentMode,
        AssessmentStatus,
        CreateAssessmentRequest,
        UpdateAssessmentRequest,
        AddAssessmentQuestionBody,
        AddAssessmentQuestionResponse,
        CreateSectionBody,
        UpdateSectionBody,
        ListAssessmentsResponse,
        CountAssessmentsResponse,
        GenerateAssessmentResponse,
        Attempt,
        AttemptResponse,
        AttemptPresentation,
        ApiError,
        FieldError,
        Session,
        SessionKind,
        SessionStatus,
        QuestionRating,
        UserTagRating,
        LoginBody,
        AuthResponse,
        RegisterBody,
        StartLearningBody,
        StartLearningResponse,
        UserInfo,
        MeResponse,
        CreateKeyResponse,
        UpdateAgentBody,
        AgentSummary,
        AgentTokenSummary,
        CreateAgentBody,
        CreateAgentTokenBody,
        CreateSessionBody,
        CreateSessionResponse,
        GetSessionResponse,
        GetSessionQuestion,
        AnswerSessionBody,
        AnswerSessionResponse,
        FinishSessionResponse,
        ListMySessionsResponse,
        SessionSummary,
        PatchSessionBody,
        GradeOutcome,
        SessionResult,
        StudyPlan,
        CreatePlanBody,
        ListUsersResponse,
        PatchUserAdminBody,
        TokenEntry,
        ListTokensResponse,
        ListAuditLogsResponse,
        AdminHealthResponse,
        AdminAssessmentEntry,
        AdminAssessmentDetail,
        AdminPreviewQuestion,
        AdminListAssessmentsResponse,
        AuditLogEntry,
        AgentActivityResponse,
        ActivityEntry,
        RunResponse,
        PendingAttemptRow,
        DistributionBucket,
        ItemStats,
        AssessmentStatsParams,
        AssessmentStatsResponse,
        ListAttemptsResponse,
        CohortStatsResponse,
        UpdateSettingsBody,
        crate::settings::EffectiveSettings,
        crate::settings::RateLimitSettings,
        crate::settings::QuotaSettings,
        crate::settings::TierLimit,
        crate::settings::TierQuota,
        crate::http::explore::ExploreResponse,
        crate::http::explore::ExploreFacetsResponse,
        crate::http::explore::ExploreCounts,
        DeepDive,
        CreateDeepDiveBody,
        ListDeepDivesQuery,
        ListDeepDivesResponse,
        PatchDeepDiveBody,
        PublishDeepDiveBody,
        DeepDiveRevision,
        DeepDiveRevisionsResponse
    )),
    info(
        title = "ame API",
        version = "0.2.0",
        description = "Adaptive Mastery Engine — assessment platform with agent identity."
    )
)]
pub struct ApiDoc;

pub fn openapi_yaml() -> String {
    use utoipa::OpenApi;
    ApiDoc::openapi()
        .to_yaml()
        .expect("OpenAPI spec serializes to YAML")
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/openapi.yaml", get(|| async { openapi_yaml() }))
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
