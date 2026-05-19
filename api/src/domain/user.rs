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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Scope {
    #[serde(rename = "human")]
    Human,
    #[serde(rename = "agent:write-questions")]
    AgentWriteQuestions,
    #[serde(rename = "agent:read-only")]
    AgentReadOnly,
}

impl AsRef<str> for Scope {
    fn as_ref(&self) -> &str {
        match self {
            Scope::Human => "human",
            Scope::AgentWriteQuestions => "agent:write-questions",
            Scope::AgentReadOnly => "agent:read-only",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub token_hash: String,
    pub scopes: Vec<String>, // postgres text[] maps to Vec<String>
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_used_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub revoked_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}
