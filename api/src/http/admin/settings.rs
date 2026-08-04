use axum::{Json, extract::State, response::IntoResponse};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{
    auth::admin::RequireAdmin,
    domain::error::{ApiError, FieldError},
    http::AppState,
};

/// Partial update for platform settings — any omitted field is left unchanged.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsBody {
    pub maintenance_mode: Option<bool>,
    pub ratelimit: Option<crate::settings::RateLimitSettings>,
}

/// GET /api/v1/admin/settings — current effective platform settings (config
/// defaults overlaid with any tb_settings overrides).
#[utoipa::path(
    get,
    path = "/api/v1/admin/settings",
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
    _admin: RequireAdmin,
) -> Result<impl IntoResponse, ApiError> {
    let settings = crate::settings::get_effective(&state.pool, &state.valkey, &state.config).await;
    Ok(Json(settings))
}

/// PUT /api/v1/admin/settings — upsert one or more setting overrides. Each changed
/// key is written to tb_settings, the settings cache is busted, and one audit
/// row is emitted. Returns the new effective settings.
#[utoipa::path(
    put,
    path = "/api/v1/admin/settings",
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
    admin: RequireAdmin,
    Json(body): Json<UpdateSettingsBody>,
) -> Result<impl IntoResponse, ApiError> {
    let actor = admin.0.user.id;

    if let Some(ratelimit) = &body.ratelimit {
        for (tier, limits) in [("free", &ratelimit.free), ("premium", &ratelimit.premium)] {
            if limits.burst == 0 || limits.rate == 0 {
                return Err(ApiError::Validation(vec![FieldError {
                    field: format!("ratelimit.{tier}"),
                    message: "burst and rate must both be greater than zero".into(),
                }]));
            }
        }
    }

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

    for (key, value) in &changed {
        crate::settings::upsert(&state.pool, key, value, actor)
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
