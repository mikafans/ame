use std::str::FromStr;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Admin,
    Agent,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct User {
    pub id: Uuid,
    pub owner_user_id: Option<Uuid>,
    pub email: Option<String>,
    pub display_name: String,
    pub role: Role,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// Token scope as enforced by the `api_tokens.scopes` CHECK constraint.
///
/// The wire strings (used in JSON and stored in Postgres `text[]`) must stay in
/// lock-step with the migration's CHECK list — see
/// `db/migrations/20260520210000_p7_stats_feedback_keys.sql`. `from_str` is the
/// single point of translation; anything not listed here is rejected as `UnknownScope`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Scope {
    #[serde(rename = "quiz.read")]
    QuizRead,
    #[serde(rename = "quiz.write")]
    QuizWrite,
    #[serde(rename = "attempt.read")]
    AttemptRead,
    #[serde(rename = "attempt.write")]
    AttemptWrite,
    #[serde(rename = "stats.read")]
    StatsRead,
    #[serde(rename = "feedback.write")]
    FeedbackWrite,
    #[serde(rename = "plan.read")]
    PlanRead,
    #[serde(rename = "plan.write")]
    PlanWrite,
    #[serde(rename = "public.publish")]
    PublicPublish,
    #[serde(rename = "admin")]
    Admin,
}

impl Scope {
    /// Canonical wire string for this scope.
    ///
    /// `const fn` so `ScopeConstraint::SCOPE` and `ApiError::ScopeRequired(&'static str)`
    /// can both be derived from one source of truth.
    pub const fn as_str(self) -> &'static str {
        match self {
            Scope::QuizRead => "quiz.read",
            Scope::QuizWrite => "quiz.write",
            Scope::AttemptRead => "attempt.read",
            Scope::AttemptWrite => "attempt.write",
            Scope::StatsRead => "stats.read",
            Scope::FeedbackWrite => "feedback.write",
            Scope::PlanRead => "plan.read",
            Scope::PlanWrite => "plan.write",
            Scope::PublicPublish => "public.publish",
            Scope::Admin => "admin",
        }
    }
}

impl std::fmt::Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("unknown scope: {0}")]
pub struct UnknownScope(pub String);

impl FromStr for Scope {
    type Err = UnknownScope;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "quiz.read" => Ok(Scope::QuizRead),
            "quiz.write" => Ok(Scope::QuizWrite),
            "attempt.read" => Ok(Scope::AttemptRead),
            "attempt.write" => Ok(Scope::AttemptWrite),
            "stats.read" => Ok(Scope::StatsRead),
            "feedback.write" => Ok(Scope::FeedbackWrite),
            "plan.read" => Ok(Scope::PlanRead),
            "plan.write" => Ok(Scope::PlanWrite),
            "public.publish" => Ok(Scope::PublicPublish),
            "admin" => Ok(Scope::Admin),
            other => Err(UnknownScope(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_string_roundtrip() {
        for scope in [
            Scope::QuizRead,
            Scope::QuizWrite,
            Scope::AttemptRead,
            Scope::AttemptWrite,
            Scope::StatsRead,
            Scope::FeedbackWrite,
            Scope::PlanRead,
            Scope::PlanWrite,
            Scope::PublicPublish,
            Scope::Admin,
        ] {
            let s = scope.as_str();
            let parsed: Scope = s.parse().expect("known scope should parse");
            assert_eq!(parsed, scope);
        }
    }

    #[test]
    fn scope_rejects_unknown() {
        let err = "agent:admin".parse::<Scope>().unwrap_err();
        assert_eq!(err.0, "agent:admin");
    }

    #[test]
    fn scope_display_matches_as_str() {
        assert_eq!(Scope::QuizWrite.to_string(), "quiz.write");
    }
}
