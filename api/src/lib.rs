// Forbid `.unwrap()` in production code. Test modules (`#[cfg(test)]`) and the
// integration/bench crates are exempt, where panicking on bad setup is fine.
#![cfg_attr(not(test), deny(clippy::unwrap_used))]

pub use ame_platform_application::{
    assessment, attempt, citation, deep_dive, generation, identity, note, onboarding, progress,
    question, review, source, task, templates,
};
pub use ame_platform_postgres::{
    assessment_postgres, attempt_postgres, authentication_postgres, citation_postgres,
    deep_dive_postgres, generation_postgres, identity_postgres, note_postgres, progress_postgres,
    question_postgres, review_postgres, source_postgres, task_postgres, timeline_postgres,
};
pub mod audit;
pub mod auth;
pub use ame_platform_application::{authentication, session};
pub mod config;
pub mod domain;
pub mod http;
pub mod ratelimit;
pub mod settings;

pub use ame_learning_application as learning;
pub use ame_learning_postgres as learning_postgres;
