use std::str::FromStr;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Admin,
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
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
/// `db/migrations/20260519092355_init.sql`. `from_str` is the single point of
/// translation; anything not listed here is rejected as `UnknownScope`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Scope {
    #[serde(rename = "human")]
    Human,
    #[serde(rename = "agent:write-questions")]
    AgentWriteQuestions,
    #[serde(rename = "agent:read-only")]
    AgentReadOnly,
}

impl Scope {
    /// Canonical wire string for this scope.
    ///
    /// `const fn` so `ScopeConstraint::SCOPE` and `ApiError::ScopeRequired(&'static str)`
    /// can both be derived from one source of truth.
    pub const fn as_str(self) -> &'static str {
        match self {
            Scope::Human => "human",
            Scope::AgentWriteQuestions => "agent:write-questions",
            Scope::AgentReadOnly => "agent:read-only",
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
            "human" => Ok(Scope::Human),
            "agent:write-questions" => Ok(Scope::AgentWriteQuestions),
            "agent:read-only" => Ok(Scope::AgentReadOnly),
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
            Scope::Human,
            Scope::AgentWriteQuestions,
            Scope::AgentReadOnly,
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
        assert_eq!(
            Scope::AgentWriteQuestions.to_string(),
            "agent:write-questions"
        );
    }
}
