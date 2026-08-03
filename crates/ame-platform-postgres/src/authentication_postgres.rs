//! PostgreSQL implementation of the learner-only authentication contract.

use crate::auth::token::verify_token_secret;
use crate::domain::auth::{
    AuthenticatedPrincipal, AuthenticationRepository, AuthenticationRepositoryError, PrincipalRole,
    PrincipalScope,
};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone)]
pub struct PgAuthenticationRepository {
    pool: PgPool,
}

impl PgAuthenticationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Slide the session window forward: extend a live (unrevoked) session's
    /// expiry so active learners are not logged out mid-session. A no-op if the
    /// session is already revoked or gone.
    pub async fn renew_login_session(
        &self,
        session_id: Uuid,
        expires_at: OffsetDateTime,
    ) -> Result<(), AuthenticationRepositoryError> {
        sqlx::query(
            "UPDATE tb_login_sessions SET expires_at = $2 WHERE id = $1 AND revoked_at IS NULL",
        )
        .bind(session_id)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(storage_error)?;
        Ok(())
    }

    /// Authenticate a bounded agent capability. Unlike login sessions this is
    /// absolute-expiry only: use never renews its lifetime.
    pub async fn authenticate_agent_delegation(
        &self,
        delegation_id: Uuid,
        secret: &str,
        now: OffsetDateTime,
    ) -> Result<AuthenticatedPrincipal, AuthenticationRepositoryError> {
        let row = sqlx::query(
            r#"
            SELECT d.id, d.token_hash, d.revoked_at, d.expires_at,
                   u.id AS user_id, u.email, u.display_name, u.role, u.status,
                   human.status AS identity_status, human.created_at,
                   d.actor_identity_id, agent.status AS agent_status
            FROM tb_agent_delegations d
            JOIN tb_users u ON u.id = d.subject_user_id
            JOIN tb_identities human ON human.id = u.id
            JOIN tb_identities agent ON agent.id = d.actor_identity_id
            WHERE d.id = $1 AND d.scope = 'course_author'
            "#,
        )
        .bind(delegation_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?
        .ok_or(AuthenticationRepositoryError::NotFound)?;

        validate_credential(
            row.get("token_hash"),
            row.get("revoked_at"),
            row.get("expires_at"),
            secret,
            now,
        )?;
        validate_owner_status(row.get("status"), row.get("identity_status"))?;
        if row.get::<String, _>("agent_status") != "active" {
            return Err(AuthenticationRepositoryError::OwnerInactive);
        }
        sqlx::query("UPDATE tb_agent_delegations SET last_used_at = $2 WHERE id = $1")
            .bind(delegation_id)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(storage_error)?;

        Ok(AuthenticatedPrincipal {
            user_id: row.get("user_id"),
            role: parse_role(row.get("role"))?,
            email: row.get("email"),
            display_name: row.get("display_name"),
            created_at: row.get("created_at"),
            credential_id: delegation_id,
            expires_at: row.get("expires_at"),
            actor_identity_id: row.get("actor_identity_id"),
            scope: PrincipalScope::CourseAuthor,
        })
    }
}

#[async_trait]
impl AuthenticationRepository for PgAuthenticationRepository {
    async fn authenticate_login_session(
        &self,
        session_id: Uuid,
        secret: &str,
        now: OffsetDateTime,
    ) -> Result<AuthenticatedPrincipal, AuthenticationRepositoryError> {
        let row = sqlx::query(
            r#"
            SELECT s.id, s.token_hash, s.revoked_at, s.expires_at,
                   u.id AS user_id, u.email, u.display_name,
                   u.role, u.status, i.status AS identity_status, i.created_at
            FROM tb_login_sessions s
            JOIN tb_users u ON u.id = s.user_id
            JOIN tb_identities i ON i.id = u.id
            WHERE s.id = $1
            "#,
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?
        .ok_or(AuthenticationRepositoryError::NotFound)?;

        validate_credential(
            row.get("token_hash"),
            row.get("revoked_at"),
            row.get("expires_at"),
            secret,
            now,
        )?;
        validate_owner_status(row.get("status"), row.get("identity_status"))?;

        Ok(AuthenticatedPrincipal {
            user_id: row.get("user_id"),
            role: parse_role(row.get("role"))?,
            email: row.get("email"),
            display_name: row.get("display_name"),
            created_at: row.get("created_at"),
            credential_id: row.get("id"),
            expires_at: row.get("expires_at"),
            actor_identity_id: row.get("user_id"),
            scope: PrincipalScope::Learner,
        })
    }
}

fn validate_credential(
    token_hash: String,
    revoked_at: Option<OffsetDateTime>,
    expires_at: OffsetDateTime,
    secret: &str,
    now: OffsetDateTime,
) -> Result<(), AuthenticationRepositoryError> {
    if revoked_at.is_some() {
        return Err(AuthenticationRepositoryError::Revoked);
    }
    if expires_at <= now {
        return Err(AuthenticationRepositoryError::Expired);
    }
    if !verify_token_secret(&token_hash, secret) {
        return Err(AuthenticationRepositoryError::InvalidSecret);
    }
    Ok(())
}

fn validate_owner_status(
    user_status: String,
    identity_status: String,
) -> Result<(), AuthenticationRepositoryError> {
    if user_status != "active" || identity_status != "active" {
        return Err(AuthenticationRepositoryError::OwnerInactive);
    }
    Ok(())
}

fn parse_role(role: String) -> Result<PrincipalRole, AuthenticationRepositoryError> {
    match role.as_str() {
        "learner" | "user" => Ok(PrincipalRole::Learner),
        "admin" => Ok(PrincipalRole::Admin),
        other => Err(AuthenticationRepositoryError::InvalidData(format!(
            "invalid user role: {other}"
        ))),
    }
}

fn storage_error(error: sqlx::Error) -> AuthenticationRepositoryError {
    AuthenticationRepositoryError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::PgAuthenticationRepository;
    use crate::auth::token::hash_secret;
    use crate::domain::auth::{AuthenticationRepository, PrincipalRole};
    use sqlx::migrate::Migrator;
    use sqlx::postgres::PgPoolOptions;
    use time::OffsetDateTime;
    use uuid::Uuid;

    static MIGRATOR: Migrator = sqlx::migrate!("../../db/migrations");

    #[tokio::test]
    async fn postgres_authentication_uses_clean_baseline_schema() {
        if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
            eprintln!("skipping clean PostgreSQL authentication test; set AME_RUN_DB_TESTS=1");
            return;
        }

        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .connect(&database_url)
            .await
            .expect("connect to clean PostgreSQL database");
        MIGRATOR.run(&pool).await.expect("apply clean baseline");

        let user_id = Uuid::now_v7();
        let session_id = Uuid::now_v7();
        let secret = "clean-session-secret";
        let expires_at = OffsetDateTime::now_utc() + time::Duration::hours(1);
        let mut transaction = pool.begin().await.expect("begin fixture transaction");
        sqlx::query("SET CONSTRAINTS ALL DEFERRED")
            .execute(&mut *transaction)
            .await
            .expect("defer identity constraint");
        sqlx::query(
            "INSERT INTO tb_users (id, email, email_canonical, display_name, role)
             VALUES ($1, $2, $2, $3, 'learner')",
        )
        .bind(user_id)
        .bind("auth@example.test")
        .bind("Auth Learner")
        .execute(&mut *transaction)
        .await
        .expect("insert clean user");
        sqlx::query(
            "INSERT INTO tb_identities (id, identity_type, owner_user_id, label)
             VALUES ($1, 'human', $1, $2)",
        )
        .bind(user_id)
        .bind("Auth Learner")
        .execute(&mut *transaction)
        .await
        .expect("insert clean identity");
        sqlx::query(
            "INSERT INTO tb_login_sessions (id, user_id, token_hash, expires_at)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(session_id)
        .bind(user_id)
        .bind(hash_secret(secret))
        .bind(expires_at)
        .execute(&mut *transaction)
        .await
        .expect("insert clean session");
        transaction.commit().await.expect("commit fixture");

        let principal = PgAuthenticationRepository::new(pool)
            .authenticate_login_session(session_id, secret, OffsetDateTime::now_utc())
            .await
            .expect("clean session authenticates");
        assert_eq!(principal.user_id, user_id);
        assert_eq!(principal.role, PrincipalRole::Learner);
    }
}
