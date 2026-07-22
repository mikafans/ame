//! Platform-wide domain contracts shared by applications and adapters.

pub mod assessment;
pub mod attempt;
pub mod auth;
pub mod deep_dive;
pub mod generation;
pub mod identity;
pub mod progress;
pub mod question;
pub mod quiz;
pub mod task;
pub mod timeline;
pub mod user;

/// Compatibility namespace for code being migrated from `api::domain`.
pub mod domain {
    pub use crate::{
        assessment, attempt, auth, deep_dive, generation, identity, progress, question, quiz, task,
        timeline, user,
    };
    pub use ame_learning_domain as learning;
}
