// Forbid `.unwrap()` in production code. Test modules (`#[cfg(test)]`) and the
// integration/bench crates are exempt, where panicking on bad setup is fine.
#![cfg_attr(not(test), deny(clippy::unwrap_used))]

pub mod assessment;
pub mod assessment_postgres;
pub mod attempt;
pub mod attempt_postgres;
pub mod audit;
pub mod auth;
pub mod authentication;
pub mod authentication_postgres;
pub mod config;
pub mod deep_dive;
pub mod deep_dive_postgres;
pub mod domain;
pub mod generation;
pub mod generation_postgres;
pub mod http;
pub mod identity;
pub mod identity_postgres;
pub mod onboarding;
pub mod progress;
pub mod progress_postgres;
pub mod question;
pub mod question_postgres;
pub mod ratelimit;
pub mod session;
pub mod settings;
pub mod templates;
pub mod timeline_postgres;

pub use ame_learning_application as learning;
pub use ame_learning_postgres as learning_postgres;
