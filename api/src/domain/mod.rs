//! Compatibility exports for the platform domain crate.

pub use ame_learning_domain as learning;
pub use ame_platform_domain::{
    assessment, attempt, auth, deep_dive, generation, identity, progress, question, quiz, review,
    task, timeline, user,
};
pub mod error;
