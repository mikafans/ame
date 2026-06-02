/// Quota and plan management.
use crate::domain::{error::ApiError, user::Scope};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Plan {
    Free,
    Premium,
}

#[derive(Debug, Clone, Copy)]
pub enum QuotaKind {
    AgentCreation,
}

pub async fn resolve_plan(pool: &PgPool, owner_id: Uuid) -> Result<Plan, ApiError> {
    let plan_str: String = sqlx::query_scalar("SELECT plan FROM tb_users WHERE id = $1")
        .bind(owner_id)
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    Ok(match plan_str.as_str() {
        "premium" => Plan::Premium,
        _ => Plan::Free,
    })
}

/// Returns the maximum set of scopes permitted for an agent under a given plan.
pub fn plan_scope_ceiling(plan: Plan) -> Vec<Scope> {
    match plan {
        Plan::Premium => vec![
            Scope::AssessmentRead,
            Scope::AssessmentWrite,
            Scope::AttemptRead,
            Scope::AttemptWrite,
            Scope::StatsRead,
            Scope::FeedbackWrite,
            Scope::PlanRead,
            Scope::PlanWrite,
        ],
        Plan::Free => vec![
            Scope::AssessmentRead,
            Scope::AssessmentWrite,
            Scope::AttemptRead,
            Scope::AttemptWrite,
            Scope::StatsRead,
            Scope::FeedbackWrite,
            Scope::PlanRead,
            Scope::PlanWrite,
        ],
    }
}
/// Checks if the owner has quota available for the given kind of action.
pub async fn check_quota(pool: &PgPool, owner_id: Uuid, kind: QuotaKind) -> Result<(), ApiError> {
    // 1. Resolve owner plan
    let plan = resolve_plan(pool, owner_id).await?;

    // 2. Define quota limits
    let limit = match (plan, kind) {
        (Plan::Premium, QuotaKind::AgentCreation) => 100,
        (Plan::Free, QuotaKind::AgentCreation) => 1,
    };

    // 3. Count current usage
    let usage: i64 = match kind {
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
