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
    domain::question::{
        CodePayload, CodeSnippet, CodeTest, EssayPayload, Judge, McPayload, Normalize, Question,
        QuestionKind, QuestionStatus, QuestionVersion, ShortPayload, Tag, TfPayload,
    },
    domain::{attempt::Attempt, session::Session, user::User},
    http::{
        AppState,
        admin::{ListUsersResponse, UpdateUserRoleBody},
        me::{
            CreateKeyBody, CreateKeyResponse, CreateWebhookBody, CreateWebhookResponse, KeySummary,
            ListKeysResponse, ListWebhooksResponse, RotateKeyResponse, WebhookSummary,
        },
        messages::{SendMessageBody, SendMessageResponse},
        questions::{
            CreateQuestionsBody, CreateQuestionsResponse, QuestionListResponse,
            QuestionVersionsResponse,
        },
        sessions::{
            AnswerSessionBody, AnswerSessionResponse, CreateSessionBody, CreateSessionResponse,
            FinishSessionResponse, GetSessionResponse, SessionQuestion,
        },
        stats::{DistributionBucket, ExamStatsResponse, ItemStats, QuizStatsResponse, SectionAvg},
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
        description = "Question collector + exam platform. Plan 3 surface: tags + questions CRUD with versioning."
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
        crate::http::sessions::get_session,
        crate::http::sessions::answer,
        crate::http::sessions::finish,
        crate::http::stats::quiz_stats,
        crate::http::stats::exam_stats,
        crate::http::messages::send_message,
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
        AnswerSessionBody,
        AnswerSessionResponse,
        FinishSessionResponse,
        CreateTagBody,
        CreateQuestionsBody,
        QuestionListResponse,
        CreateQuestionsResponse,
        QuestionVersionsResponse,
        QuizStatsResponse,
        ExamStatsResponse,
        DistributionBucket,
        ItemStats,
        SectionAvg,
        SendMessageBody,
        SendMessageResponse,
        User,
        ListUsersResponse,
        UpdateUserRoleBody,
        KeySummary,
        ListKeysResponse,
        CreateKeyBody,
        CreateKeyResponse,
        RotateKeyResponse,
        WebhookSummary,
        ListWebhooksResponse,
        CreateWebhookBody,
        CreateWebhookResponse,
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
