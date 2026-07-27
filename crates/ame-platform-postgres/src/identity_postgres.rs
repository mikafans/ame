//! PostgreSQL implementation of the self-host identity repository contract.

use crate::auth::token::canonical_email;
use crate::domain::identity::{
    AccountStatus, BrowserSession, CreateBrowserSession, CreateLearner, IdentityRepository,
    IdentityRepositoryError, LearnerAccount,
};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct PgIdentityRepository {
    pool: PgPool,
}

impl PgIdentityRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl IdentityRepository for PgIdentityRepository {
    async fn get_learner(&self, user_id: Uuid) -> Result<LearnerAccount, IdentityRepositoryError> {
        sqlx::query(
            r#"
            SELECT id, email_canonical, display_name, status, created_at
            FROM tb_users
            WHERE id = $1 AND role = 'learner'
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?
        .map(account_from_row)
        .ok_or(IdentityRepositoryError::AccountNotFound)
    }

    async fn find_learner_by_email(
        &self,
        email: &str,
    ) -> Result<Option<LearnerAccount>, IdentityRepositoryError> {
        sqlx::query(
            r#"
            SELECT id, email_canonical, display_name, status, created_at
            FROM tb_users
            WHERE email_canonical = $1 AND role = 'learner'
            "#,
        )
        .bind(canonical_email(email))
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(account_from_row))
        .map_err(storage_error)
    }

    async fn create_learner(
        &self,
        input: CreateLearner,
    ) -> Result<LearnerAccount, IdentityRepositoryError> {
        if input.email.trim().is_empty() {
            return Err(IdentityRepositoryError::EmptyField { field: "email" });
        }
        if input.display_name.trim().is_empty() {
            return Err(IdentityRepositoryError::EmptyField {
                field: "display_name",
            });
        }

        let id = Uuid::now_v7();
        let email = canonical_email(&input.email);
        let mut transaction = self.pool.begin().await.map_err(storage_error)?;
        sqlx::query("SET CONSTRAINTS ALL DEFERRED")
            .execute(&mut *transaction)
            .await
            .map_err(storage_error)?;

        let row = sqlx::query(
            r#"
            INSERT INTO tb_users (id, email, email_canonical, display_name, role)
            VALUES ($1, $2, $3, $4, 'learner')
            RETURNING id, email_canonical, display_name, status, created_at
            "#,
        )
        .bind(id)
        .bind(&email)
        .bind(&email)
        .bind(input.display_name.trim())
        .fetch_one(&mut *transaction)
        .await
        .map_err(|error| {
            if is_unique_violation(&error) {
                IdentityRepositoryError::AccountAlreadyExists
            } else {
                storage_error(error)
            }
        })?;

        sqlx::query(
            "INSERT INTO tb_identities (id, identity_type, owner_user_id, label)
             VALUES ($1, 'human', $1, $2)",
        )
        .bind(id)
        .bind(input.display_name.trim())
        .execute(&mut *transaction)
        .await
        .map_err(storage_error)?;

        transaction.commit().await.map_err(storage_error)?;
        Ok(account_from_row(row))
    }

    async fn create_browser_session(
        &self,
        input: CreateBrowserSession,
    ) -> Result<BrowserSession, IdentityRepositoryError> {
        if input.token_hash.trim().is_empty() {
            return Err(IdentityRepositoryError::EmptyField {
                field: "token_hash",
            });
        }

        let session_id = Uuid::now_v7();
        sqlx::query(
            r#"
            INSERT INTO tb_login_sessions (id, user_id, token_hash, expires_at)
            SELECT $1, id, $2, $3
            FROM tb_users
            WHERE id = $4
            RETURNING id, user_id, token_hash, expires_at, revoked_at
            "#,
        )
        .bind(session_id)
        .bind(&input.token_hash)
        .bind(input.expires_at)
        .bind(input.user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| {
            if is_unique_violation(&error) {
                IdentityRepositoryError::Storage("browser session token already exists".into())
            } else {
                storage_error(error)
            }
        })?
        .map(session_from_row)
        .ok_or(IdentityRepositoryError::AccountNotFound)
    }

    async fn get_browser_session(
        &self,
        session_id: Uuid,
    ) -> Result<BrowserSession, IdentityRepositoryError> {
        sqlx::query(
            "SELECT id, user_id, token_hash, expires_at, revoked_at
             FROM tb_login_sessions
             WHERE id = $1",
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?
        .map(session_from_row)
        .ok_or(IdentityRepositoryError::SessionNotFound)
    }
}

fn storage_error(error: sqlx::Error) -> IdentityRepositoryError {
    IdentityRepositoryError::storage(error)
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .and_then(|database_error| database_error.code())
        .as_deref()
        == Some("23505")
}

fn account_from_row(row: sqlx::postgres::PgRow) -> LearnerAccount {
    LearnerAccount {
        id: row.get("id"),
        email: row.get("email_canonical"),
        display_name: row.get("display_name"),
        status: match row.get::<String, _>("status").as_str() {
            "deactivated" => AccountStatus::Deactivated,
            _ => AccountStatus::Active,
        },
        created_at: row.get("created_at"),
    }
}

fn session_from_row(row: sqlx::postgres::PgRow) -> BrowserSession {
    BrowserSession {
        id: row.get("id"),
        user_id: row.get("user_id"),
        token_hash: row.get("token_hash"),
        expires_at: row.get("expires_at"),
        revoked_at: row.get("revoked_at"),
    }
}

#[cfg(test)]
mod tests {
    use super::PgIdentityRepository;
    use crate::identity::{IdentityContractFixtures, exercise_identity_repository_contract};
    use sqlx::migrate::Migrator;
    use sqlx::postgres::PgPoolOptions;

    static MIGRATOR: Migrator = sqlx::migrate!("../../db/migrations");

    #[tokio::test]
    async fn postgres_repository_satisfies_identity_contract_on_clean_database() {
        if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
            eprintln!("skipping clean PostgreSQL identity contract test; set AME_RUN_DB_TESTS=1");
            return;
        }

        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must point to a clean PostgreSQL database");
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .connect(&database_url)
            .await
            .expect("connect to clean PostgreSQL database");
        MIGRATOR
            .run(&pool)
            .await
            .expect("apply clean learning baseline");

        exercise_identity_repository_contract(
            &PgIdentityRepository::new(pool),
            IdentityContractFixtures::default(),
        )
        .await;
    }
}
