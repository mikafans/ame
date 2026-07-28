//! Platform application services, repository ports, and in-memory adapters.

pub mod assessment;
pub mod attempt;
pub mod authentication;
pub mod deep_dive;
pub mod generation;
pub mod identity;
pub mod onboarding;
pub mod progress;
pub mod question;
pub mod review;
pub mod session;
pub mod task;
pub mod templates;
pub mod token;

pub mod auth {
    pub mod token {
        pub use crate::token::*;
    }
}

pub mod domain {
    pub use ame_learning_domain as learning;
    pub use ame_platform_domain::{
        assessment, attempt, auth, deep_dive, generation, identity, progress, question, quiz,
        review, task, timeline, user,
    };
}

pub mod learning {
    pub use ame_learning_application::*;
    pub use ame_learning_domain::*;
}
