//! Compatibility exports for the platform domain crate.

pub use ame_learning_domain as learning;
pub use ame_platform_domain::{
    assessment, attempt, auth, citation, deep_dive, generation, identity, note, progress, question,
    quiz, review, source, task, timeline, user,
};
pub mod error;
