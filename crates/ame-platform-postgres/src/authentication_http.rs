use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RegisteredUser {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: String,
}

#[derive(Debug, Clone)]
pub struct LoginUser {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub password_hash: Option<String>,
    pub status: String,
    pub identity_status: String,
}

pub async fn register_user(
    pool: &PgPool,
    user_id: Uuid,
    email: &str,
    display_name: &str,
    password_hash: &str,
) -> Result<RegisteredUser, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    sqlx::query("SET CONSTRAINTS ALL DEFERRED")
        .execute(&mut *transaction)
        .await?;
    let row = sqlx::query(
        "INSERT INTO tb_users (id, email, email_canonical, display_name, role, password_hash)
         VALUES ($1, $2, $2, $3, 'learner', $4)
         RETURNING id, email_canonical, display_name, role",
    )
    .bind(user_id)
    .bind(email)
    .bind(display_name)
    .bind(password_hash)
    .fetch_one(&mut *transaction)
    .await?;
    sqlx::query(
        "INSERT INTO tb_identities (id, identity_type, owner_user_id, label)
         VALUES ($1, 'human', $1, $2)",
    )
    .bind(user_id)
    .bind(display_name)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(RegisteredUser {
        id: row.get("id"),
        email: row.get("email_canonical"),
        display_name: row.get("display_name"),
        role: row.get("role"),
    })
}

pub async fn find_login_user(pool: &PgPool, email: &str) -> Result<Option<LoginUser>, sqlx::Error> {
    sqlx::query(
        "SELECT u.id, u.email_canonical, u.display_name, u.role, u.password_hash, u.status,
                i.status AS identity_status
         FROM tb_users u JOIN tb_identities i ON i.id = u.id
         WHERE u.email_canonical = $1",
    )
    .bind(email)
    .fetch_optional(pool)
    .await
    .map(|row| {
        row.map(|row| LoginUser {
            id: row.get("id"),
            email: row.get("email_canonical"),
            display_name: row.get("display_name"),
            role: row.get("role"),
            password_hash: row.get("password_hash"),
            status: row.get("status"),
            identity_status: row.get("identity_status"),
        })
    })
}

pub async fn issue_login_session(
    pool: &PgPool,
    token_id: Uuid,
    user_id: Uuid,
    token_hash: &str,
    expires_at: OffsetDateTime,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO tb_login_sessions (id, user_id, token_hash, expires_at)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(token_id)
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at)
    .execute(pool)
    .await
    .map(|_| ())
}

pub async fn revoke_login_session(pool: &PgPool, token_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE tb_login_sessions SET revoked_at = NOW() WHERE id = $1")
        .bind(token_id)
        .execute(pool)
        .await
        .map(|_| ())
}
