//! OpenAPI 3 specification for the current learner contract.

use utoipa::OpenApi;

use crate::http::{
    admin::{
        AdminHealthResponse, AuditLogEntry, ListAuditLogsResponse, ListUsersResponse,
        PatchUserAdminBody, UpdateSettingsBody,
    },
    assessments::{
        AssessmentActivityQuery, AssessmentItemBody, AssessmentOptionResponse,
        AssessmentQuestionResponse, AssessmentResponse, CreateAssessmentBody,
    },
    attempts::{AttemptItemResponse, AttemptResponse, SaveAnswerBody, StartAttemptBody},
    auth::{AuthResponse, LoginBody, RegisterBody, UserInfo},
    deep_dives::{CreateDeepDiveBody, DeepDiveActivityQuery, DeepDiveResponse},
    generation::{GenerationRunResponse, StartGenerationRunBody, TransitionGenerationRunBody},
    learning::{
        FinishLearningSessionBody, LearningActivityResponse, LearningGoalResponse,
        LearningJourneyResponse, LearningJourneySummaryResponse, LearningObjectiveResponse,
        LearningRecommendationResponse, LearningResponseBody, LearningSessionResponse,
    },
    me::MeResponse,
    onboarding::{
        PreviewActivityResponse, PreviewLearningBody, PreviewLearningResponse,
        PreviewObjectiveResponse, StartLearningBody, StartLearningResponse,
    },
    progress::{
        EvidenceBody, EvidenceResponse, ObjectiveActivityBody, RecommendationBody,
        RecommendationResponse, SnapshotResponse, StreakBody, StreakResponse,
        TimelineEventResponse,
    },
    questions::{CreateQuestionBody, QuestionResponse},
};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::http::auth::register,
        crate::http::auth::login,
        crate::http::auth::logout,
        crate::http::onboarding::start_learning,
        crate::http::onboarding::preview_learning,
        crate::http::learning::get_journey,
        crate::http::learning::list_journeys,
        crate::http::learning::get_learning_session,
        crate::http::learning::finish_learning_session,
        crate::http::learning::start_activity,
        crate::http::learning::author_activity_content,
        crate::http::questions::create_question,
        crate::http::questions::create_question_version,
        crate::http::questions::get_question_version,
        crate::http::assessments::create_assessment,
        crate::http::assessments::get_assessment_for_activity,
        crate::http::assessments::get_assessment,
        crate::http::attempts::start_attempt,
        crate::http::attempts::get_attempt,
        crate::http::attempts::list_journey_attempts,
        crate::http::attempts::save_answer,
        crate::http::attempts::finish_attempt,
        crate::http::progress::record_evidence,
        crate::http::progress::snapshot,
        crate::http::progress::recommend,
        crate::http::progress::record_streak,
        crate::http::progress::list_streaks,
        crate::http::progress::list_timeline,
        crate::http::deep_dives::create,
        crate::http::deep_dives::get_for_activity,
        crate::http::deep_dives::get_one,
        crate::http::generation::start,
        crate::http::generation::get_one,
        crate::http::generation::transition,
        crate::http::me::get_me,
        crate::http::admin::list_users,
        crate::http::admin::patch_user_admin,
        crate::http::admin::list_audit_logs,
        crate::http::admin::get_admin_health,
        crate::http::admin::get_settings,
        crate::http::admin::put_settings
    ),
    components(schemas(
        crate::domain::error::ApiError,
        crate::domain::error::FieldError,
        crate::domain::user::Role,
        crate::domain::user::User,
        LoginBody,
        AuthResponse,
        RegisterBody,
        UserInfo,
        PreviewLearningBody,
        PreviewLearningResponse,
        PreviewObjectiveResponse,
        PreviewActivityResponse,
        StartLearningBody,
        StartLearningResponse,
        MeResponse,
        LearningGoalResponse,
        LearningJourneyResponse,
        LearningJourneySummaryResponse,
        LearningRecommendationResponse,
        LearningObjectiveResponse,
        LearningActivityResponse,
        LearningSessionResponse,
        FinishLearningSessionBody,
        LearningResponseBody,
        CreateQuestionBody,
        QuestionResponse,
        CreateAssessmentBody,
        AssessmentActivityQuery,
        AssessmentQuestionResponse,
        AssessmentOptionResponse,
        AssessmentItemBody,
        AssessmentResponse,
        StartAttemptBody,
        SaveAnswerBody,
        AttemptResponse,
        AttemptItemResponse,
        EvidenceBody,
        EvidenceResponse,
        ObjectiveActivityBody,
        RecommendationBody,
        RecommendationResponse,
        SnapshotResponse,
        StreakBody,
        StreakResponse,
        TimelineEventResponse,
        CreateDeepDiveBody,
        DeepDiveResponse,
        DeepDiveActivityQuery,
        GenerationRunResponse,
        StartGenerationRunBody,
        TransitionGenerationRunBody,
        ListUsersResponse,
        PatchUserAdminBody,
        ListAuditLogsResponse,
        AuditLogEntry,
        AdminHealthResponse,
        UpdateSettingsBody,
        crate::settings::EffectiveSettings,
        crate::settings::RateLimitSettings,
        crate::settings::TierLimit
    )),
    info(
        title = "ame API",
        version = "0.3.0",
        description = "ame — an agent-friendly learning API with one owner-scoped learner model."
    )
)]
pub struct ApiDoc;

pub fn openapi_yaml() -> String {
    ApiDoc::openapi()
        .to_yaml()
        .expect("OpenAPI spec serializes to YAML")
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
