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

impl FromStr for Role {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user" => Ok(Role::User),
            "admin" => Ok(Role::Admin),
            "agent" => Ok(Role::Agent),
            _ => Err(format!("unknown role: {s}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: Uuid,
    pub email: Option<String>,
    pub display_name: String,
    pub role: Role,
    pub status: UserStatus,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
    Active,
    Deactivated,
}

/// Token scope as enforced by the `api_tokens.scopes` CHECK constraint.
///
/// The wire strings (used in JSON and stored in Postgres `text[]`) must stay in
/// lock-step with the migration's CHECK list — see
/// `db/migrations/20260520210000_p7_stats_feedback_keys.sql`. `from_str` is the
/// single point of translation; anything not listed here is rejected as `UnknownScope`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub enum Scope {
    /// Read assessments and their questions.
    #[serde(rename = "assessment.read")]
    AssessmentRead,
    /// Create, edit, and publish assessments.
    #[serde(rename = "assessment.write")]
    AssessmentWrite,
    /// View their own attempts and results.
    #[serde(rename = "attempt.read")]
    AttemptRead,
    /// Start sessions and submit answers.
    #[serde(rename = "attempt.write")]
    AttemptWrite,
    /// Read aggregate statistics.
    #[serde(rename = "stats.read")]
    StatsRead,
    /// Post feedback/ratings on questions.
    #[serde(rename = "feedback.write")]
    FeedbackWrite,
    /// Read study plans.
    #[serde(rename = "plan.read")]
    PlanRead,
    /// Create and edit study plans.
    #[serde(rename = "plan.write")]
    PlanWrite,
    /// Full access.
    #[serde(rename = "admin")]
    Admin,
}

impl Scope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Scope::AssessmentRead => "assessment.read",
            Scope::AssessmentWrite => "assessment.write",
            Scope::AttemptRead => "attempt.read",
            Scope::AttemptWrite => "attempt.write",
            Scope::StatsRead => "stats.read",
            Scope::FeedbackWrite => "feedback.write",
            Scope::PlanRead => "plan.read",
            Scope::PlanWrite => "plan.write",
            Scope::Admin => "admin",
        }
    }

    /// Whether an agent token may hold this scope. Agents reach the platform
    /// through the run-door (`POST /v1/agents/run`); `plan.read` is grantable
    /// because it backs the agent's owner-scoped learning journey read tool.
    /// Write-only human scopes remain unavailable to agents.
    pub fn is_agent_grantable(&self) -> bool {
        !matches!(self, Scope::Admin | Scope::FeedbackWrite | Scope::PlanWrite)
    }
}

impl std::fmt::Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unknown scope: {0}")]
pub struct UnknownScope(pub String);

impl FromStr for Scope {
    type Err = UnknownScope;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "assessment.read" => Ok(Scope::AssessmentRead),
            "assessment.write" => Ok(Scope::AssessmentWrite),
            "attempt.read" => Ok(Scope::AttemptRead),
            "attempt.write" => Ok(Scope::AttemptWrite),
            "stats.read" => Ok(Scope::StatsRead),
            "feedback.write" => Ok(Scope::FeedbackWrite),
            "plan.read" => Ok(Scope::PlanRead),
            "plan.write" => Ok(Scope::PlanWrite),
            "admin" => Ok(Scope::Admin),
            _ => Err(UnknownScope(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_parsing() {
        assert_eq!(
            "assessment.read".parse::<Scope>().unwrap(),
            Scope::AssessmentRead
        );
        assert_eq!(
            "assessment.write".parse::<Scope>().unwrap(),
            Scope::AssessmentWrite
        );
        assert_eq!("admin".parse::<Scope>().unwrap(), Scope::Admin);
    }

    #[test]
    fn scope_rejects_unknown() {
        let err = "agent:admin".parse::<Scope>().unwrap_err();
        assert_eq!(err.0, "agent:admin");
    }

    #[test]
    fn scope_display_matches_as_str() {
        assert_eq!(Scope::AssessmentWrite.to_string(), "assessment.write");
    }
}
