//! Public machine-readable documents served by the API.
//!
//! The k3s front door sends `/public/*` to the API, so these documents must be
//! available from the same binary as the public API routes. Keeping the
//! checked-in documents as compile-time assets also makes an image self
//! contained and prevents a deployment from silently serving a different
//! contract than the API it documents.

use axum::{
    Router,
    body::Body,
    http::{HeaderValue, StatusCode, header::CONTENT_TYPE},
    response::Response,
    routing::get,
};

use crate::http::AppState;

const LLMS: &str = include_str!("../../../docs/public/llms.txt");
const SKILL: &str = include_str!("../../../docs/public/skill.json");
const OPENAPI: &str = include_str!("../../../docs/public/openapi.yaml");
const LEARNING_CONTRACT: &str = include_str!("../../../docs/public/learning-contract.json");
const LEARNING_PRINCIPLES: &str = include_str!("../../../docs/public/learning-principles.md");
const PYTHON_SDK: &str = include_str!("../../../sdk/python/ame.py");

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/public/llms.txt", get(llms))
        .route("/public/skill.json", get(skill))
        .route("/public/openapi.yaml", get(openapi))
        .route("/public/learning-contract.json", get(learning_contract))
        .route("/public/learning-principles.md", get(learning_principles))
        .route("/public/sdk/python/ame.py", get(python_sdk))
}

async fn llms() -> Response {
    document("text/plain; charset=utf-8", LLMS)
}

async fn skill() -> Response {
    document("application/json", SKILL)
}

async fn openapi() -> Response {
    document("application/yaml", OPENAPI)
}

async fn learning_contract() -> Response {
    document("application/json", LEARNING_CONTRACT)
}

async fn learning_principles() -> Response {
    document("text/markdown; charset=utf-8", LEARNING_PRINCIPLES)
}

async fn python_sdk() -> Response {
    document("text/x-python; charset=utf-8", PYTHON_SDK)
}

fn document(content_type: &'static str, body: &'static str) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, HeaderValue::from_static(content_type))
        .body(Body::from(body))
        .expect("static document response is valid")
}
