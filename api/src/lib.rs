// Forbid `.unwrap()` in production code. Test modules (`#[cfg(test)]`) and the
// integration/bench crates are exempt, where panicking on bad setup is fine.
#![cfg_attr(not(test), deny(clippy::unwrap_used))]

pub use ame_platform_application::{
    assessment, attempt, deep_dive, generation, identity, onboarding, progress, question, templates,
};
pub use ame_platform_postgres::{
    assessment_postgres, attempt_postgres, authentication_postgres, deep_dive_postgres,
    generation_postgres, identity_postgres, progress_postgres, question_postgres,
    timeline_postgres,
};
pub mod audit;
pub mod auth;
pub mod authentication;
pub mod config;
pub mod domain;
pub mod http;
pub mod ratelimit;
pub mod session;
pub mod settings;

pub use ame_learning_application as learning;
pub use ame_learning_postgres as learning_postgres;
