use serde_json::Value;
use sqlx::{PgPool, QueryBuilder, Row};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AuditLogRecord {
    pub id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub actor_email: Option<String>,
    pub actor_name: Option<String>,
    pub action: String,
    pub target_type: Option<String>,
    pub target_id: Option<Uuid>,
    pub target_email: Option<String>,
    pub target_name: Option<String>,
    pub metadata: Value,
    pub created_at: OffsetDateTime,
}

pub async fn list(
    pool: &PgPool,
    limit: i64,
    offset: i64,
    action: Option<&str>,
    actor_id: Option<Uuid>,
    target_id: Option<Uuid>,
) -> Result<(Vec<AuditLogRecord>, i64), sqlx::Error> {
    let action = action
        .filter(|value| !value.trim().is_empty())
        .map(str::trim);
    let mut count = QueryBuilder::new("SELECT COUNT(*) FROM tb_audit_log");
    let mut filtered = false;
    if let Some(action) = action {
        count.push(" WHERE action = ").push_bind(action);
        filtered = true;
    }
    if let Some(actor_id) = actor_id {
        count.push(if filtered {
            " AND actor_user_id = "
        } else {
            " WHERE actor_user_id = "
        });
        count.push_bind(actor_id);
        filtered = true;
    }
    if let Some(target_id) = target_id {
        count.push(if filtered {
            " AND target_id = "
        } else {
            " WHERE target_id = "
        });
        count.push_bind(target_id);
    }
    let total = count.build_query_as::<(i64,)>().fetch_one(pool).await?.0;

    let mut query = QueryBuilder::new(
        "SELECT a.id, a.actor_user_id, u_actor.email AS actor_email, u_actor.display_name AS actor_name,
                a.action, a.target_type, a.target_id, u_target.email AS target_email,
                u_target.display_name AS target_name, a.metadata, a.created_at
         FROM tb_audit_log a
         LEFT JOIN tb_users u_actor ON a.actor_user_id = u_actor.id
         LEFT JOIN tb_users u_target ON (a.target_type = 'user' AND a.target_id = u_target.id)",
    );
    let mut filtered = false;
    if let Some(action) = action {
        query.push(" WHERE a.action = ").push_bind(action);
        filtered = true;
    }
    if let Some(actor_id) = actor_id {
        query.push(if filtered {
            " AND a.actor_user_id = "
        } else {
            " WHERE a.actor_user_id = "
        });
        query.push_bind(actor_id);
        filtered = true;
    }
    if let Some(target_id) = target_id {
        query.push(if filtered {
            " AND a.target_id = "
        } else {
            " WHERE a.target_id = "
        });
        query.push_bind(target_id);
    }
    query
        .push(" ORDER BY a.created_at DESC, a.id DESC LIMIT ")
        .push_bind(limit);
    query.push(" OFFSET ").push_bind(offset);
    let rows = query.build().fetch_all(pool).await?;
    let logs = rows
        .into_iter()
        .map(|row| AuditLogRecord {
            id: row.get("id"),
            actor_user_id: row.get("actor_user_id"),
            actor_email: row.get("actor_email"),
            actor_name: row.get("actor_name"),
            action: row.get("action"),
            target_type: row.get("target_type"),
            target_id: row.get("target_id"),
            target_email: row.get("target_email"),
            target_name: row.get("target_name"),
            metadata: row.get("metadata"),
            created_at: row.get("created_at"),
        })
        .collect();
    Ok((logs, total))
}
