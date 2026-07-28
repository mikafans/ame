//! PostgreSQL adapters for the platform application contracts.

pub mod admin_users;
pub mod assessment_postgres;
pub mod attempt_postgres;
pub mod audit;
pub mod audit_query;
pub mod auth_cache;
pub mod authentication_http;
pub mod authentication_postgres;
pub mod citation_postgres;
pub mod deep_dive_postgres;
pub mod generation_postgres;
pub mod health;
pub mod idempotency;
pub mod identity_postgres;
pub mod progress_postgres;
pub mod question_postgres;
pub mod review_postgres;
pub mod settings;
pub mod source_postgres;
pub mod task_postgres;
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
pub mod citation {
    pub use ame_platform_application::citation::*;
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
pub mod review {
    pub use ame_platform_application::review::*;
}
pub mod source {
    pub use ame_platform_application::source::*;
}
pub mod task {
    pub use ame_platform_application::task::*;
}

pub mod domain {
    pub use ame_learning_domain as learning;
    pub use ame_platform_domain::{
        assessment, attempt, auth, citation, deep_dive, generation, identity, progress, question,
        quiz, review, source, task, timeline, user,
    };
}
