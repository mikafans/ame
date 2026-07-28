//! Versioned journey export/import manifests.

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::extractor::AuthenticatedUser,
    domain::error::{ApiError, FieldError},
    http::AppState,
};

const SCHEMA_VERSION: &str = "ame.journey-history.v1";

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JourneyExportManifest {
    pub schema_version: String,
    pub owner_id: Uuid,
    pub journey_id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub exported_at: OffsetDateTime,
    pub checksum: String,
    pub payload: Value,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImportReceipt {
    pub journey_id: Uuid,
    pub checksum: String,
    pub already_imported: bool,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/learning/journeys/{id}/export", get(export_journey))
        .route("/v1/learning/imports", post(import_journey))
        .with_state(state)
}

#[utoipa::path(get, path = "/api/v1/learning/journeys/{id}/export", params(("id" = Uuid, Path)), responses((status = 200, body = JourneyExportManifest)), security(("bearer" = [])), tag = "portability")]
pub async fn export_journey(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<JourneyExportManifest>, ApiError> {
    let payload = build_payload(&state.pool, auth.owner_id(), id).await?;
    Ok(Json(manifest(auth.owner_id(), id, payload)?))
}

#[utoipa::path(post, path = "/api/v1/learning/imports", request_body = JourneyExportManifest, responses((status = 200, body = ImportReceipt)), security(("bearer" = [])), tag = "portability")]
pub async fn import_journey(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(manifest): Json<JourneyExportManifest>,
) -> Result<Json<ImportReceipt>, ApiError> {
    validate_manifest(&manifest, auth.owner_id())?;
    if sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
            SELECT 1 FROM tb_portability_imports
            WHERE subject_user_id = $1 AND checksum = $2
        )",
    )
    .bind(auth.owner_id())
    .bind(&manifest.checksum)
    .fetch_one(&state.pool)
    .await
    .map_err(internal)?
    {
        return Ok(Json(receipt(&manifest, true)));
    }
    let journey_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
            SELECT 1 FROM tb_learning_journeys
            WHERE id = $1 AND subject_user_id = $2
        )",
    )
    .bind(manifest.journey_id)
    .bind(auth.owner_id())
    .fetch_one(&state.pool)
    .await
    .map_err(internal)?;
    if journey_exists {
        let current = build_payload(&state.pool, auth.owner_id(), manifest.journey_id).await?;
        if payload_checksum(&current)? != manifest.checksum {
            return Err(ApiError::IdempotencyConflict);
        }
        record_import(&state.pool, &manifest).await?;
        return Ok(Json(receipt(&manifest, true)));
    }

    let mut transaction = state.pool.begin().await.map_err(internal)?;
    for (key, table) in [
        ("goals", "tb_learning_goals"),
        ("journeys", "tb_learning_journeys"),
        ("objectives", "tb_journey_objectives"),
        ("activities", "tb_activities"),
        ("activityObjectives", "tb_activity_objectives"),
        ("generationRuns", "tb_generation_runs"),
        ("sources", "tb_sources"),
        ("sourceImportRuns", "tb_source_import_runs"),
        ("sourceSnapshots", "tb_source_snapshots"),
        ("citations", "tb_citations"),
        ("taskSubmissions", "tb_task_submissions"),
        ("evidence", "tb_mastery_evidence"),
        ("reviews", "tb_review_items"),
        ("reviewEvents", "tb_review_events"),
        ("notes", "tb_learner_notes"),
        ("variants", "tb_learning_variants"),
    ] {
        let values = manifest
            .payload
            .get(key)
            .and_then(Value::as_array)
            .ok_or_else(|| validation("payload", &format!("missing {key} array")))?;
        restore_rows(&mut transaction, table, values).await?;
    }
    sqlx::query(
        "INSERT INTO tb_portability_imports
            (id, subject_user_id, journey_id, schema_version, checksum, manifest)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(Uuid::now_v7())
    .bind(auth.owner_id())
    .bind(manifest.journey_id)
    .bind(&manifest.schema_version)
    .bind(&manifest.checksum)
    .bind(serde_json::to_value(&manifest).map_err(internal)?)
    .execute(&mut *transaction)
    .await
    .map_err(internal)?;
    transaction.commit().await.map_err(internal)?;
    Ok(Json(receipt(&manifest, false)))
}

fn manifest(
    owner_id: Uuid,
    journey_id: Uuid,
    payload: Value,
) -> Result<JourneyExportManifest, ApiError> {
    Ok(JourneyExportManifest {
        schema_version: SCHEMA_VERSION.into(),
        owner_id,
        journey_id,
        exported_at: OffsetDateTime::now_utc(),
        checksum: payload_checksum(&payload)?,
        payload,
    })
}

fn validate_manifest(manifest: &JourneyExportManifest, owner_id: Uuid) -> Result<(), ApiError> {
    if manifest.schema_version != SCHEMA_VERSION {
        return Err(validation("schemaVersion", "is not supported"));
    }
    if manifest.owner_id != owner_id {
        return Err(validation(
            "ownerId",
            "belongs to another learner and cannot be merged",
        ));
    }
    if payload_checksum(&manifest.payload)? != manifest.checksum {
        return Err(validation(
            "checksum",
            "does not match the manifest payload",
        ));
    }
    for key in [
        "goals",
        "journeys",
        "objectives",
        "activities",
        "activityObjectives",
        "generationRuns",
        "sources",
        "sourceImportRuns",
        "sourceSnapshots",
        "citations",
        "taskSubmissions",
        "evidence",
        "reviews",
        "reviewEvents",
        "notes",
        "variants",
        "history",
    ] {
        if !manifest.payload.get(key).is_some_and(Value::is_array) {
            return Err(validation("payload", &format!("missing {key} array")));
        }
        for record in manifest.payload[key].as_array().into_iter().flatten() {
            let record_owner = record
                .get("subject_user_id")
                .or_else(|| record.get("record")?.get("subject_user_id"))
                .and_then(Value::as_str);
            if let Some(record_owner) = record_owner
                && record_owner != owner_id.to_string()
            {
                return Err(validation(
                    "payload",
                    "contains records owned by another learner",
                ));
            }
        }
    }
    let journey_id = manifest.journey_id.to_string();
    if manifest.payload["journeys"]
        .as_array()
        .and_then(|journeys| journeys.first())
        .and_then(|journey| journey.get("id"))
        .and_then(Value::as_str)
        != Some(journey_id.as_str())
    {
        return Err(validation("journeyId", "does not match the journey record"));
    }
    Ok(())
}

async fn build_payload(pool: &PgPool, owner: Uuid, journey: Uuid) -> Result<Value, ApiError> {
    let journey_row = rows(
        pool,
        "SELECT to_jsonb(j) AS value FROM tb_learning_journeys j
         WHERE j.id = $1 AND j.subject_user_id = $2",
        journey,
        owner,
    )
    .await?;
    if journey_row.is_empty() {
        return Err(ApiError::NotFound {
            resource: "learning journey",
        });
    }
    let goals = rows(
        pool,
        "SELECT to_jsonb(g) AS value FROM tb_learning_goals g
         JOIN tb_learning_journeys j ON j.goal_id = g.id
         WHERE j.id = $1 AND j.subject_user_id = $2",
        journey,
        owner,
    )
    .await?;
    let objectives = rows(pool, "SELECT to_jsonb(o) AS value FROM tb_journey_objectives o WHERE o.journey_id = $1 AND o.subject_user_id = $2 ORDER BY o.order_index", journey, owner).await?;
    let activities = rows(pool, "SELECT to_jsonb(a) AS value FROM tb_activities a WHERE a.journey_id = $1 AND a.subject_user_id = $2 ORDER BY a.order_index", journey, owner).await?;
    let activity_objectives = rows(pool, "SELECT to_jsonb(ao) AS value FROM tb_activity_objectives ao JOIN tb_activities a ON a.id = ao.activity_id WHERE a.journey_id = $1 AND a.subject_user_id = $2", journey, owner).await?;
    let evidence = rows(pool, "SELECT to_jsonb(e) AS value FROM tb_mastery_evidence e WHERE e.journey_id = $1 AND e.subject_user_id = $2 ORDER BY e.created_at", journey, owner).await?;
    let reviews = rows(pool, "SELECT to_jsonb(r) AS value FROM tb_review_items r WHERE r.journey_id = $1 AND r.subject_user_id = $2 ORDER BY r.created_at", journey, owner).await?;
    let review_events = rows(pool, "SELECT to_jsonb(r) AS value FROM tb_review_events r WHERE r.journey_id = $1 AND r.subject_user_id = $2 ORDER BY r.reviewed_at", journey, owner).await?;
    let notes = rows(pool, "SELECT to_jsonb(n) AS value FROM tb_learner_notes n WHERE n.journey_id = $1 AND n.subject_user_id = $2 ORDER BY n.created_at", journey, owner).await?;
    let mut submissions = rows(pool, "SELECT to_jsonb(s) AS value FROM tb_task_submissions s WHERE s.journey_id = $1 AND s.subject_user_id = $2 ORDER BY s.created_at", journey, owner).await?;
    if let Some(archived) = archived_payload(pool, journey, owner).await? {
        let archived_submissions = archived["taskSubmissions"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        for submission in &mut submissions {
            if submission
                .get("reviewer_user_id")
                .is_some_and(Value::is_null)
                && let Some(archived_submission) = archived_submissions
                    .iter()
                    .find(|candidate| candidate.get("id") == submission.get("id"))
                && let Some(reviewer) = archived_submission.get("reviewer_user_id")
            {
                submission["reviewer_user_id"] = reviewer.clone();
            }
        }
    }
    let variants = rows(pool, "SELECT to_jsonb(v) AS value FROM tb_learning_variants v JOIN tb_activities a ON a.id = v.source_activity_id WHERE a.journey_id = $1 AND v.subject_user_id = $2 ORDER BY v.created_at", journey, owner).await?;
    let history = history_rows(pool, journey, owner).await?;
    let owner_all = |table: &'static str| async move {
        let query = format!(
            "SELECT to_jsonb(t) AS value FROM {table} t WHERE t.subject_user_id = $1 ORDER BY t.created_at"
        );
        sqlx::query(&query)
            .bind(owner)
            .fetch_all(pool)
            .await
            .map_err(internal)
            .map(|rows| {
                rows.into_iter()
                    .map(|row| row.get("value"))
                    .collect::<Vec<Value>>()
            })
    };
    let generation_runs = owner_all("tb_generation_runs").await?;
    let sources = owner_all("tb_sources").await?;
    let source_import_runs = owner_all("tb_source_import_runs").await?;
    let source_snapshots = owner_all("tb_source_snapshots").await?;
    let citations = owner_all("tb_citations").await?;
    Ok(json!({
        "goals": goals,
        "journeys": journey_row,
        "objectives": objectives,
        "activities": activities,
        "activityObjectives": activity_objectives,
        "generationRuns": generation_runs,
        "sources": sources,
        "sourceImportRuns": source_import_runs,
        "sourceSnapshots": source_snapshots,
        "citations": citations,
        "taskSubmissions": submissions,
        "evidence": evidence,
        "reviews": reviews,
        "reviewEvents": review_events,
        "notes": notes,
        "variants": variants,
        "history": history,
    }))
}

async fn history_rows(pool: &PgPool, journey: Uuid, owner: Uuid) -> Result<Vec<Value>, ApiError> {
    let sessions = rows(pool, "SELECT jsonb_build_object('kind', 'learning_session', 'record', to_jsonb(s)) AS value FROM tb_learning_sessions s WHERE s.journey_id = $1 AND s.subject_user_id = $2 ORDER BY s.created_at", journey, owner).await?;
    let attempts = rows(pool, "SELECT jsonb_build_object('kind', 'assessment_attempt', 'record', to_jsonb(a)) AS value FROM tb_attempts a WHERE a.journey_id = $1 AND a.subject_user_id = $2 ORDER BY a.created_at", journey, owner).await?;
    let live = sessions.into_iter().chain(attempts).collect::<Vec<_>>();
    if !live.is_empty() {
        return Ok(live);
    }
    archived_payload(pool, journey, owner).await.map(|payload| {
        payload
            .and_then(|payload| payload["history"].as_array().cloned())
            .unwrap_or_default()
    })
}

async fn archived_payload(
    pool: &PgPool,
    journey: Uuid,
    owner: Uuid,
) -> Result<Option<Value>, ApiError> {
    sqlx::query_scalar::<_, Value>(
        "SELECT manifest->'payload'
         FROM tb_portability_imports
         WHERE journey_id = $1 AND subject_user_id = $2
         ORDER BY imported_at DESC LIMIT 1",
    )
    .bind(journey)
    .bind(owner)
    .fetch_optional(pool)
    .await
    .map_err(internal)
}

async fn rows(
    pool: &PgPool,
    query: &str,
    journey: Uuid,
    owner: Uuid,
) -> Result<Vec<Value>, ApiError> {
    sqlx::query(query)
        .bind(journey)
        .bind(owner)
        .fetch_all(pool)
        .await
        .map_err(internal)
        .map(|rows| rows.into_iter().map(|row| row.get("value")).collect())
}

async fn restore_rows(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    table: &str,
    values: &[Value],
) -> Result<(), ApiError> {
    if values.is_empty() {
        return Ok(());
    }
    let allowed = [
        "tb_learning_goals",
        "tb_learning_journeys",
        "tb_journey_objectives",
        "tb_activities",
        "tb_activity_objectives",
        "tb_generation_runs",
        "tb_sources",
        "tb_source_import_runs",
        "tb_source_snapshots",
        "tb_citations",
        "tb_task_submissions",
        "tb_mastery_evidence",
        "tb_review_items",
        "tb_review_events",
        "tb_learner_notes",
        "tb_learning_variants",
    ];
    if !allowed.contains(&table) {
        return Err(ApiError::Internal(anyhow::anyhow!("invalid restore table")));
    }
    let conflict_query = if table == "tb_activity_objectives" {
        format!(
            "SELECT EXISTS (
                SELECT 1 FROM jsonb_array_elements($1::jsonb) value
                JOIN {table} t
                  ON t.activity_id = (value->>'activity_id')::uuid
                 AND t.objective_id = (value->>'objective_id')::uuid
                WHERE to_jsonb(t) <> value
            )"
        )
    } else {
        format!(
            "SELECT EXISTS (
                SELECT 1 FROM jsonb_array_elements($1::jsonb) value
                JOIN {table} t ON t.id = (value->>'id')::uuid
                WHERE to_jsonb(t) <> value
            )"
        )
    };
    if sqlx::query_scalar::<_, bool>(&conflict_query)
        .bind(Value::Array(values.to_vec()))
        .fetch_one(&mut **transaction)
        .await
        .map_err(internal)?
    {
        return Err(ApiError::IdempotencyConflict);
    }
    let record_expression = if table == "tb_task_submissions" {
        "jsonb_set(value, '{reviewer_user_id}', 'null'::jsonb)"
    } else {
        "value"
    };
    let query = format!(
        "INSERT INTO {table}
         SELECT (jsonb_populate_record(NULL::{table}, {record_expression})).*
         FROM jsonb_array_elements($1::jsonb) value
         ON CONFLICT DO NOTHING"
    );
    sqlx::query(&query)
        .bind(Value::Array(values.to_vec()))
        .execute(&mut **transaction)
        .await
        .map_err(internal)?;
    Ok(())
}

async fn record_import(pool: &PgPool, manifest: &JourneyExportManifest) -> Result<(), ApiError> {
    sqlx::query(
        "INSERT INTO tb_portability_imports
            (id, subject_user_id, journey_id, schema_version, checksum, manifest)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (subject_user_id, checksum) DO NOTHING",
    )
    .bind(Uuid::now_v7())
    .bind(manifest.owner_id)
    .bind(manifest.journey_id)
    .bind(&manifest.schema_version)
    .bind(&manifest.checksum)
    .bind(serde_json::to_value(manifest).map_err(internal)?)
    .execute(pool)
    .await
    .map_err(internal)?;
    Ok(())
}

fn payload_checksum(payload: &Value) -> Result<String, ApiError> {
    serde_json::to_vec(payload)
        .map(|bytes| hex::encode(Sha256::digest(bytes)))
        .map_err(internal)
}

fn receipt(manifest: &JourneyExportManifest, already_imported: bool) -> ImportReceipt {
    ImportReceipt {
        journey_id: manifest.journey_id,
        checksum: manifest.checksum.clone(),
        already_imported,
    }
}

fn validation(field: &str, message: &str) -> ApiError {
    ApiError::Validation(vec![FieldError {
        field: field.into(),
        message: message.into(),
    }])
}

fn internal(error: impl std::fmt::Display) -> ApiError {
    ApiError::Internal(anyhow::anyhow!(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::{migrate::Migrator, postgres::PgPoolOptions};

    static MIGRATOR: Migrator = sqlx::migrate!("../db/migrations");

    #[test]
    fn manifest_rejects_tampering_foreign_owners_and_partial_payloads() {
        let owner = Uuid::now_v7();
        let journey = Uuid::now_v7();
        let payload = json!({
            "goals": [], "journeys": [{"id": journey, "subject_user_id": owner}],
            "objectives": [], "activities": [], "activityObjectives": [],
            "generationRuns": [], "sources": [], "sourceImportRuns": [],
            "sourceSnapshots": [], "citations": [], "taskSubmissions": [],
            "evidence": [], "reviews": [], "reviewEvents": [], "notes": [], "variants": [], "history": []
        });
        let mut value = manifest(owner, journey, payload).unwrap();
        assert!(validate_manifest(&value, owner).is_ok());
        assert!(validate_manifest(&value, Uuid::now_v7()).is_err());
        value.payload["notes"] = json!([{"subject_user_id": Uuid::now_v7()}]);
        assert!(validate_manifest(&value, owner).is_err());
        value.payload.as_object_mut().unwrap().remove("notes");
        value.checksum = payload_checksum(&value.payload).unwrap();
        assert!(validate_manifest(&value, owner).is_err());
    }

    #[tokio::test]
    async fn journey_manifest_round_trips_and_repeated_restore_is_idempotent() {
        if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
            eprintln!("skipping portability integration test; set AME_RUN_DB_TESTS=1");
            return;
        }
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"))
            .await
            .unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        let owner = Uuid::now_v7();
        let goal = Uuid::now_v7();
        let journey = Uuid::now_v7();
        let objective = Uuid::now_v7();
        let activity = Uuid::now_v7();
        let mut transaction = pool.begin().await.unwrap();
        sqlx::query("SET CONSTRAINTS ALL DEFERRED")
            .execute(&mut *transaction)
            .await
            .unwrap();
        sqlx::query("INSERT INTO tb_users (id, email, email_canonical, display_name) VALUES ($1, $2, $2, 'Portable Learner')")
            .bind(owner).bind(format!("portable-{owner}@example.test"))
            .execute(&mut *transaction).await.unwrap();
        sqlx::query("INSERT INTO tb_identities (id, identity_type, owner_user_id, label) VALUES ($1, 'human', $1, 'Portable Learner')")
            .bind(owner).execute(&mut *transaction).await.unwrap();
        sqlx::query("INSERT INTO tb_learning_goals (id, subject_user_id, source_actor_id, raw_intent, normalized_statement) VALUES ($1, $2, $2, 'learn', 'Learn')")
            .bind(goal).bind(owner).execute(&mut *transaction).await.unwrap();
        sqlx::query("INSERT INTO tb_learning_journeys (id, goal_id, subject_user_id, source_actor_id, promise) VALUES ($1, $2, $3, $3, 'Resume me')")
            .bind(journey).bind(goal).bind(owner).execute(&mut *transaction).await.unwrap();
        sqlx::query("INSERT INTO tb_journey_objectives (id, journey_id, subject_user_id, verb, statement, success_criteria, order_index) VALUES ($1, $2, $3, 'explain', 'Portable state', 'Resume it', 0)")
            .bind(objective).bind(journey).bind(owner).execute(&mut *transaction).await.unwrap();
        sqlx::query("INSERT INTO tb_activities (id, journey_id, subject_user_id, source_actor_id, kind, title, order_index) VALUES ($1, $2, $3, $3, 'explanation', 'Portable activity', 0)")
            .bind(activity).bind(journey).bind(owner).execute(&mut *transaction).await.unwrap();
        sqlx::query(
            "INSERT INTO tb_activity_objectives (activity_id, objective_id) VALUES ($1, $2)",
        )
        .bind(activity)
        .bind(objective)
        .execute(&mut *transaction)
        .await
        .unwrap();
        transaction.commit().await.unwrap();

        let payload = build_payload(&pool, owner, journey).await.unwrap();
        let exported = manifest(owner, journey, payload).unwrap();
        sqlx::query("DELETE FROM tb_learning_journeys WHERE id = $1")
            .bind(journey)
            .execute(&pool)
            .await
            .unwrap();
        for _ in 0..2 {
            let mut transaction = pool.begin().await.unwrap();
            for (key, table) in [
                ("goals", "tb_learning_goals"),
                ("journeys", "tb_learning_journeys"),
                ("objectives", "tb_journey_objectives"),
                ("activities", "tb_activities"),
                ("activityObjectives", "tb_activity_objectives"),
            ] {
                restore_rows(
                    &mut transaction,
                    table,
                    exported.payload[key].as_array().unwrap(),
                )
                .await
                .unwrap();
            }
            transaction.commit().await.unwrap();
        }
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM tb_learning_journeys WHERE id = $1 AND subject_user_id = $2",
            )
            .bind(journey)
            .bind(owner)
            .fetch_one(&pool)
            .await
            .unwrap(),
            1
        );
        assert_eq!(
            payload_checksum(&build_payload(&pool, owner, journey).await.unwrap()).unwrap(),
            exported.checksum
        );
    }
}
