//! OpenAPI 3 spec generation for the ame API.
//!
//! Each handler annotates its inputs/outputs with `#[utoipa::path]`; this
//! module aggregates the annotated paths and the `ToSchema`-derived types
//! into a single [`ApiDoc`]. The generated spec is served at `/openapi.json`
//! and snapshotted to `api/openapi.yaml` — the snapshot is checked into git
//! and validated by [`openapi_yaml_snapshot_matches`] so a route that drifts
//! from the committed contract fails CI rather than going unnoticed.
//!
//! ### Updating the snapshot
//!
//! When you intentionally change a route or schema, regenerate the snapshot:
//!
//! ```text
//! make openapi
//! ```
//!
//! which writes `api/openapi.yaml`; commit the diff alongside the source
//! change.

use axum::{Json, Router, routing::get};
use utoipa::{
    Modify, OpenApi,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};

use crate::{
    bank::questions::{QuestionFilter, QuestionInsert, QuestionPatch},
    domain::exam::{Exam, ExamMethod, ExamSection, ExamStatus},
    domain::question::{
        CodePayload, CodeSnippet, CodeTest, EssayPayload, Judge, McPayload, Normalize, Question,
        QuestionKind, QuestionStatus, QuestionVersion, ShortPayload, Tag, TfPayload,
    },
    domain::{attempt::Attempt, session::Session, user::User},
    engine::planner::{StudyPlan, StudyPlanItem, StudyPlanWeek},
    http::{
        AppState,
        admin::{
            AuditLogEntry, ListAuditLogsResponse, ListUsersResponse, ModerateBody,
            PatchUserAdminBody, UpdateUserRoleBody,
        },
        me::{
            AgentSummary, CohortStatsBucket, CohortStatsQuery, CohortStatsResponse,
            CreateAgentBody, CreateAgentResponse, CreateKeyBody, CreateKeyResponse,
            CreateWebhookBody, CreateWebhookResponse, KeySummary, ListAgentsResponse,
            ListKeysResponse, ListWebhooksResponse, RotateKeyResponse, UpdateAgentBody,
            WebhookSummary,
        },
        messages::{SendMessageBody, SendMessageResponse},
        plans::CreatePlanBody,
        questions::{
            CreateQuestionsBody, CreateQuestionsResponse, QuestionListResponse,
            QuestionVersionsResponse,
        },
        sessions::{
            AnswerSessionBody, AnswerSessionResponse, CreateSessionBody, CreateSessionResponse,
            FinishSessionResponse, GetSessionResponse, GradeAttemptBody, ListMySessionsResponse,
            PendingAttemptRow, SessionQuestion, SessionSummary,
        },
        stats::{
            DistributionBucket, ExamStatsResponse, ItemStats, MeStatsResponse, QuizStatsResponse,
            SectionAvg,
        },
        tags::CreateTagBody,
    },
};

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi
            .components
            .get_or_insert_with(utoipa::openapi::Components::default);
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("uuid_secret")
                    .build(),
            ),
        );
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "ame API",
        version = "0.1.0",
        description = "Question collector + exam platform."
    ),
    paths(
        crate::http::tags::list_tags,
        crate::http::tags::create_tag,
        crate::http::questions::list_questions,
        crate::http::questions::get_question,
        crate::http::questions::list_versions,
        crate::http::questions::create_questions,
        crate::http::questions::update_question,
        crate::http::questions::promote_question,
        crate::http::questions::archive_question,
        crate::http::sessions::create_session,
        crate::http::sessions::list_my_sessions,
        crate::http::sessions::get_session,
        crate::http::sessions::patch_session,
        crate::http::sessions::answer,
        crate::http::sessions::autosave_answers,
        crate::http::sessions::finish,
        crate::http::sessions::list_pending_attempts,
        crate::http::sessions::grade_attempt,
        crate::http::stats::quiz_stats,
        crate::http::stats::exam_stats,
        crate::http::stats::me_stats,
        crate::http::messages::send_message,
        crate::http::plans::create_plan,
        crate::http::plans::get_plan,
        crate::http::quizzes::list_quizzes,
        crate::http::quizzes::get_quiz,
        crate::http::quizzes::patch_quiz,
        crate::http::quizzes::create_quiz,
        crate::http::quizzes::add_quiz_question,
        crate::http::quizzes::count_quizzes,
        crate::http::quizzes::generate_quiz,
        crate::http::quizzes::explore,
        crate::http::me::get_me,
        crate::http::export::export_data,
        crate::http::me::list_agents,
        crate::http::me::create_agent,
        crate::http::me::update_agent,
        crate::http::me::delete_agent,
        crate::http::me::list_attempts,
        crate::http::me::get_cohort_stats,
        crate::http::exams::compose_exam,
        crate::http::exams::list_exams,
        crate::http::exams::get_exam,
        crate::http::exams::patch_exam_status,
        crate::http::admin::list_users,
        crate::http::admin::patch_user_admin,
        crate::http::admin::update_user_role,
        crate::http::admin::deactivate_user,
        crate::http::admin::list_audit_logs,
        crate::http::admin::moderate_quiz,
    ),
    components(schemas(
        Tag,
        Question,
        QuestionVersion,
        QuestionKind,
        QuestionStatus,
        Normalize,
        Judge,
        CodeSnippet,
        McPayload,
        TfPayload,
        ShortPayload,
        EssayPayload,
        CodePayload,
        CodeTest,
        QuestionInsert,
        QuestionPatch,
        QuestionFilter,
        Session,
        Attempt,
        SessionQuestion,
        CreateSessionBody,
        CreateSessionResponse,
        GetSessionResponse,
        crate::http::sessions::PatchSessionBody,
        AnswerSessionBody,
        AnswerSessionResponse,
        FinishSessionResponse,
        SessionSummary,
        ListMySessionsResponse,
        PendingAttemptRow,
        GradeAttemptBody,
        CreateTagBody,
        CreateQuestionsBody,
        QuestionListResponse,
        CreateQuestionsResponse,
        QuestionVersionsResponse,
        QuizStatsResponse,
        ExamStatsResponse,
        MeStatsResponse,
        DistributionBucket,
        ItemStats,
        SectionAvg,
        SendMessageBody,
        SendMessageResponse,
        User,
        ListUsersResponse,
        UpdateUserRoleBody,
        PatchUserAdminBody,
        AuditLogEntry,
        ListAuditLogsResponse,
        ModerateBody,
        KeySummary,
        ListKeysResponse,
        CreateKeyBody,
        CreateKeyResponse,
        RotateKeyResponse,
        AgentSummary,
        crate::http::export::ExportResponse,
        ListAgentsResponse,
        CreateAgentBody,
        CreateAgentResponse,
        UpdateAgentBody,
        WebhookSummary,
        ListWebhooksResponse,
        CreateWebhookBody,
        CreateWebhookResponse,
        CohortStatsQuery,
        CohortStatsBucket,
        CohortStatsResponse,
        StudyPlan,
        StudyPlanWeek,
        StudyPlanItem,
        CreatePlanBody,
        crate::http::quizzes::QuizSummary,
        crate::http::quizzes::ListQuizzesResponse,
        crate::http::quizzes::ExploreResponse,
        crate::http::quizzes::GetQuizResponse,
        crate::http::quizzes::QuizQuestion,
        crate::http::quizzes::QuizPatch,
        crate::http::quizzes::PatchQuizResponse,
        crate::http::quizzes::GenerateBody,
        crate::http::quizzes::GenerateResponse,
        crate::http::quizzes::CreateQuizBody,
        crate::http::quizzes::CreateQuizResponse,
        crate::http::quizzes::CreatedQuiz,
        crate::http::quizzes::AddQuizQuestionBody,
        crate::http::quizzes::AddQuizQuestionResponse,
        crate::http::quizzes::CountQuizzesQuery,
        crate::http::quizzes::CountQuizzesResponse,
        crate::http::me::MeResponse,
        crate::http::me::ListAttemptsResponse,
        Exam,
        ExamMethod,
        ExamStatus,
        ExamSection,
        crate::http::exams::ComposeExamBody,
        crate::http::exams::ComposeExamResponse,
        crate::http::exams::GetExamResponse,
        crate::http::exams::ListExamsResponse,
        crate::http::exams::SectionSpec,
        crate::http::exams::PatchExamStatusBody,
        crate::http::exams::PatchExamStatusResponse,
    )),
    modifiers(&SecurityAddon),
)]
pub struct ApiDoc;

/// Path to the committed yaml snapshot, anchored at the api crate root.
pub const SNAPSHOT_PATH: &str = "openapi.yaml";

/// Generate the OpenAPI yaml for the current source tree.
pub fn openapi_yaml() -> String {
    serde_yaml::to_string(&ApiDoc::openapi()).expect("OpenAPI yaml serialization is infallible")
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/openapi.json", get(|| async { Json(ApiDoc::openapi()) }))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fail if `api/openapi.yaml` drifts from the generated spec. Run
    /// `make openapi` (or `cargo run --bin gen-openapi` once that bin exists)
    /// to refresh after an intentional change.
    #[test]
    fn openapi_yaml_snapshot_matches() {
        let generated = openapi_yaml();
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/openapi.yaml");
        let committed = std::fs::read_to_string(path).unwrap_or_else(|_| {
            panic!("missing openapi.yaml snapshot at {path}; run `make openapi` to create it")
        });

        if generated.trim() != committed.trim() {
            eprintln!("--- generated ---\n{generated}\n--- committed ---\n{committed}");
            panic!("openapi.yaml is out of date with the source. Regenerate via `make openapi`.");
        }
    }
}
