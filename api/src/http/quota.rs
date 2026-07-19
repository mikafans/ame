/// Local quota management.
use crate::domain::error::ApiError;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaKind {
    AgentCreation,
    Assessment,
    Question,
}

/// Checks if the owner has quota available for the given kind of action.
pub async fn check_quota(
    pool: &PgPool,
    valkey: Option<&deadpool_redis::Pool>,
    config: &crate::config::Config,
    owner_id: Uuid,
    kind: QuotaKind,
    add_count: i64,
) -> Result<(), ApiError> {
    // Define quota limits from the effective settings. The clean baseline has
    // no subscription column, so every actor uses the local free-policy tier.
    let quota = match valkey {
        Some(vk) => crate::settings::get_effective(pool, vk, config).await.quota,
        None => crate::settings::EffectiveSettings::from_config(config).quota,
    };
    let limit = match kind {
        QuotaKind::AgentCreation => quota.agents.free,
        QuotaKind::Assessment => quota.assessments.free,
        QuotaKind::Question => quota.questions.free,
    };

    // 3. Count current usage
    let mut conn = pool
        .acquire()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    crate::http::db::set_rls_guc(&mut conn, owner_id, false).await?;

    let usage: i64 = match kind {
        QuotaKind::AgentCreation => sqlx::query_scalar(
            "SELECT COUNT(*) FROM tb_agents WHERE owner_user_id = $1 AND revoked_at IS NULL",
        )
        .bind(owner_id)
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?,
        QuotaKind::Assessment => sqlx::query_scalar(
            "SELECT COUNT(*) FROM tb_assessments WHERE owner_id = $1 AND deleted_at IS NULL",
        )
        .bind(owner_id)
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?,
        QuotaKind::Question => sqlx::query_scalar(
            "SELECT COUNT(*) FROM tb_questions WHERE owner_id = $1 AND status != 'archived'",
        )
        .bind(owner_id)
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?,
    };

    if usage + add_count > limit {
        let kind_str = match kind {
            QuotaKind::AgentCreation => "agent_creation",
            QuotaKind::Assessment => "assessment",
            QuotaKind::Question => "question",
        };

        metrics::counter!("quota_rejection_total", "kind" => kind_str).increment(1);

        if let Some(vk) = valkey {
            let conn_res = vk.get().await;
            if let Ok(mut c) = conn_res {
                let _: Result<(), _> = redis::cmd("INCR")
                    .arg("ame:quota_rejections_count")
                    .query_async(&mut *c)
                    .await;
            }
        }

        return Err(ApiError::QuotaExceeded {
            kind: kind_str.to_string(),
            limit,
            usage,
        });
    }

    Ok(())
}
