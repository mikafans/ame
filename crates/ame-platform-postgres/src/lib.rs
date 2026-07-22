//! PostgreSQL adapters for the platform application contracts.

pub mod assessment_postgres;
pub mod attempt_postgres;
pub mod authentication_postgres;
pub mod deep_dive_postgres;
pub mod generation_postgres;
pub mod identity_postgres;
pub mod progress_postgres;
pub mod question_postgres;
pub mod timeline_postgres;

pub mod assessment {
    pub use ame_platform_application::assessment::*;
}
pub mod attempt {
    pub use ame_platform_application::attempt::*;
}
pub mod auth {
    pub mod token {
        pub use ame_platform_application::token::*;
    }
}
pub mod deep_dive {
    pub use ame_platform_application::deep_dive::*;
}
pub mod generation {
    pub use ame_platform_application::generation::*;
}
pub mod identity {
    pub use ame_platform_application::identity::*;
}
pub mod progress {
    pub use ame_platform_application::progress::*;
}
pub mod question {
    pub use ame_platform_application::question::*;
}

pub mod domain {
    pub use ame_platform_domain::{
        assessment, attempt, auth, deep_dive, generation, identity, progress, question, quiz, task,
        timeline, user,
    };
    pub use ame_learning_domain as learning;
}
