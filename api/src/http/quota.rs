//! Quota and plan management.

use crate::domain::error::ApiError;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Plan {
    Free,
    Premium,
}

#[derive(Debug, Clone, Copy)]
pub enum QuotaKind {
    PublicQuiz,
    AgentCreation,
}

/// Checks if the owner has quota available for the given kind of action.
pub async fn check_quota(pool: &PgPool, owner_id: Uuid, kind: QuotaKind) -> Result<(), ApiError> {
    // 1. Resolve owner plan
    let plan_str: String = sqlx::query_scalar("SELECT plan FROM tb_users WHERE id = $1")
        .bind(owner_id)
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let plan: Plan = match plan_str.as_str() {
        "premium" => Plan::Premium,
        _ => Plan::Free,
    };

    // 2. Define quota limits
    let limit = match (plan, kind) {
        (Plan::Premium, QuotaKind::PublicQuiz) => 1000,
        (Plan::Free, QuotaKind::PublicQuiz) => 5,
        (Plan::Premium, QuotaKind::AgentCreation) => 100,
        (Plan::Free, QuotaKind::AgentCreation) => 1,
    };

    // 3. Count current usage
    let usage: i64 = match kind {
        QuotaKind::PublicQuiz => sqlx::query_scalar(
            "SELECT COUNT(*) FROM tb_quizzes q 
                 WHERE q.created_by IN (SELECT id FROM tb_users WHERE id = $1 OR owner_user_id = $1)
                 AND q.visibility = 'public'",
        )
        .bind(owner_id)
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?,

        QuotaKind::AgentCreation => sqlx::query_scalar(
            "SELECT COUNT(*) FROM tb_users WHERE owner_user_id = $1 AND role = 'agent'",
        )
        .bind(owner_id)
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?,
    };

    if usage >= limit {
        let kind_str = match kind {
            QuotaKind::PublicQuiz => "public_quiz",
            QuotaKind::AgentCreation => "agent_creation",
        };
        return Err(ApiError::QuotaExceeded {
            kind: kind_str.to_string(),
            limit,
            usage,
        });
    }

    Ok(())
}
