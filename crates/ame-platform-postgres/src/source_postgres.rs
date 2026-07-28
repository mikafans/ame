//! PostgreSQL source snapshot adapter.

use crate::{
    domain::source::{ImportSource, SourceError, SourceImportStatus, SourceKind, validate_import},
    source::{Source, SourceImportRun, SourceRepository, SourceSnapshot, content_sha256},
};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct PgSourceRepository {
    pool: PgPool,
}

impl PgSourceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SourceRepository for PgSourceRepository {
    async fn import(
        &self,
        input: ImportSource,
    ) -> Result<(Source, SourceImportRun, SourceSnapshot), SourceError> {
        validate_import(&input)?;
        let digest = content_sha256(&input.content);
        let mut transaction = self.pool.begin().await.map_err(storage)?;
        if let Some(row) = sqlx::query(
            r#"SELECT r.id AS run_id, r.source_id, r.subject_user_id, r.retry_key, r.status,
                      r.error_code, r.created_at AS run_created_at, r.completed_at,
                      s.source_kind, s.locator, s.created_at AS source_created_at,
                      p.id AS snapshot_id, p.media_type, p.content_sha256,
                      p.byte_length, p.content, p.created_at AS snapshot_created_at
                 FROM tb_source_import_runs r
                 JOIN tb_sources s ON s.id = r.source_id
                 LEFT JOIN tb_source_snapshots p ON p.import_run_id = r.id
                WHERE r.subject_user_id = $1 AND r.retry_key = $2"#,
        )
        .bind(input.subject_user_id)
        .bind(&input.retry_key)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(storage)?
        {
            if row.get::<String, _>("status") != "completed" {
                return Err(SourceError::RetryConflict);
            }
            let existing_digest: Option<String> = row.try_get("content_sha256").ok();
            if existing_digest.as_deref() != Some(digest.as_str())
                || row.get::<String, _>("locator") != input.locator
            {
                return Err(SourceError::RetryConflict);
            }
            return Ok(map_import(&row));
        }
        let source_id = Uuid::now_v7();
        let run_id = Uuid::now_v7();
        let snapshot_id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO tb_sources (id, subject_user_id, source_kind, locator)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(source_id)
        .bind(input.subject_user_id)
        .bind(kind(input.source_kind))
        .bind(&input.locator)
        .execute(&mut *transaction)
        .await
        .map_err(storage)?;
        sqlx::query(
            "INSERT INTO tb_source_import_runs
                (id, source_id, subject_user_id, retry_key, status, completed_at)
             VALUES ($1, $2, $3, $4, 'completed', now())",
        )
        .bind(run_id)
        .bind(source_id)
        .bind(input.subject_user_id)
        .bind(&input.retry_key)
        .execute(&mut *transaction)
        .await
        .map_err(storage)?;
        sqlx::query(
            "INSERT INTO tb_source_snapshots
                (id, source_id, import_run_id, subject_user_id, media_type,
                 content_sha256, byte_length, content)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(snapshot_id)
        .bind(source_id)
        .bind(run_id)
        .bind(input.subject_user_id)
        .bind(&input.media_type)
        .bind(&digest)
        .bind(input.content.len() as i32)
        .bind(&input.content)
        .execute(&mut *transaction)
        .await
        .map_err(storage)?;
        transaction.commit().await.map_err(storage)?;
        let snapshot = self
            .get_snapshot(input.subject_user_id, snapshot_id)
            .await?;
        Ok((
            Source {
                id: source_id,
                subject_user_id: input.subject_user_id,
                kind: input.source_kind,
                locator: input.locator,
                created_at: snapshot.created_at,
            },
            SourceImportRun {
                id: run_id,
                source_id,
                retry_key: input.retry_key,
                status: SourceImportStatus::Completed,
                error_code: None,
                created_at: snapshot.created_at,
                completed_at: Some(snapshot.created_at),
            },
            snapshot,
        ))
    }

    async fn record_failure(
        &self,
        subject_user_id: Uuid,
        source_kind: SourceKind,
        locator: String,
        retry_key: String,
        error_code: String,
    ) -> Result<SourceImportRun, SourceError> {
        if locator.trim().is_empty() {
            return Err(SourceError::EmptyField { field: "locator" });
        }
        if retry_key.trim().is_empty() {
            return Err(SourceError::EmptyField { field: "retry_key" });
        }
        if error_code.trim().is_empty() {
            return Err(SourceError::EmptyField {
                field: "error_code",
            });
        }
        let mut transaction = self.pool.begin().await.map_err(storage)?;
        if let Some(row) = sqlx::query(
            "SELECT r.*, s.source_kind, s.locator FROM tb_source_import_runs r
             JOIN tb_sources s ON s.id = r.source_id
             WHERE r.subject_user_id = $1 AND r.retry_key = $2",
        )
        .bind(subject_user_id)
        .bind(&retry_key)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(storage)?
        {
            if row.get::<String, _>("source_kind") != kind(source_kind)
                || row.get::<String, _>("locator") != locator
            {
                return Err(SourceError::RetryConflict);
            }
            return Ok(map_run(&row));
        }
        let source_id = Uuid::now_v7();
        let run_id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO tb_sources (id, subject_user_id, source_kind, locator)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(source_id)
        .bind(subject_user_id)
        .bind(kind(source_kind))
        .bind(&locator)
        .execute(&mut *transaction)
        .await
        .map_err(storage)?;
        let row = sqlx::query(
            "INSERT INTO tb_source_import_runs
                (id, source_id, subject_user_id, retry_key, status, error_code, completed_at)
             VALUES ($1, $2, $3, $4, 'failed', $5, now())
             RETURNING *",
        )
        .bind(run_id)
        .bind(source_id)
        .bind(subject_user_id)
        .bind(retry_key)
        .bind(error_code)
        .fetch_one(&mut *transaction)
        .await
        .map_err(storage)?;
        let run = map_run(&row);
        transaction.commit().await.map_err(storage)?;
        Ok(run)
    }

    async fn list_imports(
        &self,
        subject_user_id: Uuid,
    ) -> Result<Vec<SourceImportRun>, SourceError> {
        sqlx::query(
            "SELECT * FROM tb_source_import_runs
             WHERE subject_user_id = $1 ORDER BY created_at DESC",
        )
        .bind(subject_user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(storage)
        .map(|rows| rows.iter().map(map_run).collect())
    }

    async fn get_snapshot(
        &self,
        subject_user_id: Uuid,
        snapshot_id: Uuid,
    ) -> Result<SourceSnapshot, SourceError> {
        sqlx::query("SELECT * FROM tb_source_snapshots WHERE id = $1 AND subject_user_id = $2")
            .bind(snapshot_id)
            .bind(subject_user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .map(|row| map_snapshot(&row))
            .ok_or(SourceError::NotFound)
    }

    async fn list_snapshots(
        &self,
        subject_user_id: Uuid,
    ) -> Result<Vec<SourceSnapshot>, SourceError> {
        sqlx::query(
            "SELECT * FROM tb_source_snapshots
             WHERE subject_user_id = $1 ORDER BY created_at DESC",
        )
        .bind(subject_user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(storage)
        .map(|rows| rows.iter().map(map_snapshot).collect())
    }
}

fn map_snapshot(row: &sqlx::postgres::PgRow) -> SourceSnapshot {
    SourceSnapshot {
        id: row.get("id"),
        source_id: row.get("source_id"),
        import_run_id: row.get("import_run_id"),
        subject_user_id: row.get("subject_user_id"),
        media_type: row.get("media_type"),
        content_sha256: row.get("content_sha256"),
        byte_length: row.get::<i32, _>("byte_length") as u32,
        content: row.get("content"),
        created_at: row.get("created_at"),
    }
}

fn map_run(row: &sqlx::postgres::PgRow) -> SourceImportRun {
    SourceImportRun {
        id: row.get("id"),
        source_id: row.get("source_id"),
        retry_key: row.get("retry_key"),
        status: match row.get::<String, _>("status").as_str() {
            "requested" => SourceImportStatus::Requested,
            "completed" => SourceImportStatus::Completed,
            _ => SourceImportStatus::Failed,
        },
        error_code: row.get("error_code"),
        created_at: row.get("created_at"),
        completed_at: row.get("completed_at"),
    }
}

fn map_import(row: &sqlx::postgres::PgRow) -> (Source, SourceImportRun, SourceSnapshot) {
    let source_id = row.get("source_id");
    let snapshot = SourceSnapshot {
        id: row.get("snapshot_id"),
        source_id,
        import_run_id: row.get("run_id"),
        subject_user_id: row.get("subject_user_id"),
        media_type: row.get("media_type"),
        content_sha256: row.get("content_sha256"),
        byte_length: row.get::<i32, _>("byte_length") as u32,
        content: row.get("content"),
        created_at: row.get("snapshot_created_at"),
    };
    (
        Source {
            id: source_id,
            subject_user_id: snapshot.subject_user_id,
            kind: parse_kind(row.get::<String, _>("source_kind").as_str()),
            locator: row.get("locator"),
            created_at: row.get("source_created_at"),
        },
        SourceImportRun {
            id: row.get("run_id"),
            source_id,
            retry_key: row.get("retry_key"),
            status: SourceImportStatus::Completed,
            error_code: row.get("error_code"),
            created_at: row.get("run_created_at"),
            completed_at: row.get("completed_at"),
        },
        snapshot,
    )
}

fn kind(value: SourceKind) -> &'static str {
    match value {
        SourceKind::Url => "url",
        SourceKind::Document => "document",
        SourceKind::LocalFile => "local_file",
    }
}

fn parse_kind(value: &str) -> SourceKind {
    match value {
        "url" => SourceKind::Url,
        "document" => SourceKind::Document,
        _ => SourceKind::LocalFile,
    }
}

fn storage(error: impl std::fmt::Display) -> SourceError {
    SourceError::Storage(error.to_string())
}
