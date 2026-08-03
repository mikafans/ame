use ame_platform_domain::user::{Role, User, UserStatus};
use sqlx::{PgPool, QueryBuilder, Row};
use uuid::Uuid;

pub async fn list(
    pool: &PgPool,
    limit: i64,
    offset: i64,
    search: Option<&str>,
) -> Result<(Vec<User>, i64), sqlx::Error> {
    let search = search
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!("%{}%", value.trim()));
    let mut count = QueryBuilder::new("SELECT COUNT(*) FROM tb_users");
    if let Some(search) = &search {
        count.push(" WHERE email ILIKE ").push_bind(search.clone());
        count
            .push(" OR display_name ILIKE ")
            .push_bind(search.clone());
    }
    let total = count.build_query_as::<(i64,)>().fetch_one(pool).await?.0;

    let mut users = QueryBuilder::new(
        "SELECT u.id, u.email_canonical AS email, u.display_name, u.role, u.plan, u.status, u.created_at FROM tb_users u",
    );
    if let Some(search) = &search {
        users
            .push(" WHERE u.email ILIKE ")
            .push_bind(search.clone());
        users
            .push(" OR u.display_name ILIKE ")
            .push_bind(search.clone());
    }
    users
        .push(" ORDER BY u.created_at DESC, u.id DESC LIMIT ")
        .push_bind(limit);
    users.push(" OFFSET ").push_bind(offset);
    let rows = users.build().fetch_all(pool).await?;
    let users = rows
        .into_iter()
        .map(|row| User {
            id: row.get("id"),
            email: row.get("email"),
            display_name: row.get("display_name"),
            role: if row.get::<String, _>("role") == "admin" {
                Role::Admin
            } else {
                Role::User
            },
            plan: row.get("plan"),
            status: if row.get::<String, _>("status") == "deactivated" {
                UserStatus::Deactivated
            } else {
                UserStatus::Active
            },
            created_at: row.get("created_at"),
        })
        .collect();
    Ok((users, total))
}

pub async fn role_status(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Option<(String, String)>, sqlx::Error> {
    sqlx::query_as("SELECT role, status FROM tb_users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
}

pub async fn active_admin_count(pool: &PgPool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT COUNT(*) FROM tb_users WHERE role = 'admin' AND status = 'active'")
        .fetch_one(pool)
        .await
}

pub async fn update_status(pool: &PgPool, user_id: Uuid, status: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE tb_users SET status = $1 WHERE id = $2")
        .bind(status)
        .bind(user_id)
        .execute(pool)
        .await
        .map(|_| ())
}

pub async fn update_role(pool: &PgPool, user_id: Uuid, role: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE tb_users SET role = $1 WHERE id = $2")
        .bind(role)
        .bind(user_id)
        .execute(pool)
        .await
        .map(|_| ())
}

pub async fn update_plan(pool: &PgPool, user_id: Uuid, plan: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE tb_users SET plan = $1 WHERE id = $2")
        .bind(plan)
        .bind(user_id)
        .execute(pool)
        .await
        .map(|_| ())
}
