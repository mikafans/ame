use axum::{extract::FromRequestParts, http::request::Parts};
use sqlx::{
    PgConnection, PgPool, Postgres,
    pool::PoolConnection,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use uuid::Uuid;

use crate::{auth::extractor::AuthenticatedUser, domain::error::ApiError, http::AppState};

pub struct DbConn(pub PoolConnection<Postgres>);

impl std::ops::Deref for DbConn {
    type Target = PgConnection;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for DbConn {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub async fn set_rls_guc(
    conn: &mut sqlx::PgConnection,
    owner_id: Uuid,
    is_admin: bool,
) -> Result<(), crate::domain::error::ApiError> {
    let owner_id_str = owner_id.to_string();
    let is_admin_str = if is_admin { "true" } else { "false" };

    sqlx::query("SELECT set_config('app.owner', $1, false), set_config('app.is_admin', $2, false)")
        .bind(&owner_id_str)
        .bind(is_admin_str)
        .execute(&mut *conn)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    Ok(())
}

impl FromRequestParts<AppState> for DbConn {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth = AuthenticatedUser::from_request_parts(parts, state).await?;

        let mut conn = state
            .pool
            .acquire()
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;

        let is_admin = auth.user.role == crate::domain::user::Role::Admin;
        set_rls_guc(&mut conn, auth.owner_id, is_admin).await?;

        Ok(DbConn(conn))
    }
}

pub fn convert_pool_to_ame_app(pool: &PgPool) -> PgPool {
    let opts = pool.connect_options();
    let mut app_opts: PgConnectOptions = (*opts).clone();
    app_opts = app_opts.username("ame_app").password("postgres");

    PgPoolOptions::new()
        .max_connections(5)
        .after_release(|conn: &mut PgConnection, _meta| {
            Box::pin(async move {
                sqlx::query("RESET ALL;").execute(conn).await?;
                Ok(true)
            })
        })
        .connect_lazy_with(app_opts)
}
