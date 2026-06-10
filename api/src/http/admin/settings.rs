use axum::{Json, extract::State, response::IntoResponse};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{auth::scope::RequireScope, domain::error::ApiError, http::AppState};

use super::AdminScope;

/// Partial update for platform settings — any omitted field is left unchanged.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsBody {
    pub maintenance_mode: Option<bool>,
    pub ratelimit: Option<crate::settings::RateLimitSettings>,
    pub quota: Option<crate::settings::QuotaSettings>,
}

/// GET /v1/admin/settings — current effective platform settings (config
/// defaults overlaid with any tb_settings overrides).
#[utoipa::path(
    get,
    path = "/v1/admin/settings",
    responses(
        (status = 200, description = "Effective platform settings", body = crate::settings::EffectiveSettings),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (Admin required)"),
    ),
    security(("bearer" = [])),
    tag = "admin"
)]
pub async fn get_settings(
    State(state): State<AppState>,
    _admin: RequireScope<AdminScope>,
) -> Result<impl IntoResponse, ApiError> {
    let settings = crate::settings::get_effective(&state.pool, &state.valkey, &state.config).await;
    Ok(Json(settings))
}

/// PUT /v1/admin/settings — upsert one or more setting overrides. Each changed
/// key is written to tb_settings, the settings cache is busted, and one audit
/// row is emitted. Returns the new effective settings.
#[utoipa::path(
    put,
    path = "/v1/admin/settings",
    request_body = UpdateSettingsBody,
    responses(
        (status = 200, description = "Updated effective platform settings", body = crate::settings::EffectiveSettings),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (Admin required)"),
    ),
    security(("bearer" = [])),
    tag = "admin"
)]
pub async fn put_settings(
    State(state): State<AppState>,
    admin: RequireScope<AdminScope>,
    Json(body): Json<UpdateSettingsBody>,
) -> Result<impl IntoResponse, ApiError> {
    let actor = admin.0.user.id;

    let mut changed: Vec<(&'static str, serde_json::Value)> = Vec::new();
    if let Some(m) = body.maintenance_mode {
        changed.push(("maintenance_mode", serde_json::json!(m)));
    }
    if let Some(rl) = body.ratelimit {
        changed.push((
            "ratelimit",
            serde_json::to_value(rl).map_err(|e| ApiError::Internal(e.into()))?,
        ));
    }
    if let Some(q) = body.quota {
        changed.push((
            "quota",
            serde_json::to_value(q).map_err(|e| ApiError::Internal(e.into()))?,
        ));
    }

    for (key, value) in &changed {
        sqlx::query(
            "INSERT INTO tb_settings (key, value, updated_by, updated_at) \
             VALUES ($1, $2, $3, now()) \
             ON CONFLICT (key) DO UPDATE \
             SET value = EXCLUDED.value, updated_by = EXCLUDED.updated_by, updated_at = now()",
        )
        .bind(key)
        .bind(value)
        .bind(actor)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

        crate::audit::audit(
            state.pool.clone(),
            Some(actor),
            "settings.update",
            Some("setting"),
            None,
            serde_json::json!({ "key": key, "value": value }),
        );
    }

    // Bust the cache so the next read (and the maintenance/rate-limit
    // middlewares) sees the change immediately, not after the TTL.
    crate::settings::invalidate(&state.valkey).await;

    let settings = crate::settings::get_effective(&state.pool, &state.valkey, &state.config).await;
    Ok(Json(settings))
}
