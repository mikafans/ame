// Forbid `.unwrap()` in production code. Test modules (`#[cfg(test)]`) and the
// integration/bench crates are exempt, where panicking on bad setup is fine.
#![cfg_attr(not(test), deny(clippy::unwrap_used))]

pub use ame_platform_application::{
    assessment, attempt, deep_dive, generation, identity, onboarding, progress, question, templates,
};
pub mod assessment_postgres;
pub mod attempt_postgres;
pub mod audit;
pub mod auth;
pub mod authentication;
pub mod authentication_postgres;
pub mod config;
pub mod deep_dive_postgres;
pub mod domain;
pub mod generation_postgres;
pub mod http;
pub mod identity_postgres;
pub mod progress_postgres;
pub mod question_postgres;
pub mod ratelimit;
pub mod session;
pub mod settings;
pub mod timeline_postgres;

pub use ame_learning_application as learning;
pub use ame_learning_postgres as learning_postgres;
