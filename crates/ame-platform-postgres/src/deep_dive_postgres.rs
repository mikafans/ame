//! PostgreSQL implementation of the evidence-linked deep-dive contract.

use crate::{
    deep_dive::DeepDiveRepository,
    domain::{
        deep_dive::{CreateDeepDive, DeepDive, DeepDiveError, validate_deep_dive},
        generation::{GenerationRepository, validate_content_run},
        question::ContentReviewStatus,
    },
};
use async_trait::async_trait;
use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct PgDeepDiveRepository {
    pool: PgPool,
}

impl PgDeepDiveRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn ensure_owned_evidence(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
        activity_id: Uuid,
        objective_id: Uuid,
        evidence_id: Uuid,
    ) -> Result<(), DeepDiveError> {
        let owned = sqlx::query_scalar::<_, bool>(
            r#"SELECT EXISTS (
                    SELECT 1
                      FROM tb_mastery_evidence e
                      JOIN tb_learning_journeys j
                        ON j.id = e.journey_id
                       AND j.subject_user_id = e.subject_user_id
                      JOIN tb_activities a
                        ON a.id = e.activity_id
                       AND a.journey_id = e.journey_id
                       AND a.subject_user_id = e.subject_user_id
                      JOIN tb_journey_objectives o
                        ON o.id = e.objective_id
                       AND o.journey_id = e.journey_id
                       AND o.subject_user_id = e.subject_user_id
                     WHERE e.id = $1
                       AND e.subject_user_id = $2
                       AND e.journey_id = $3
                       AND e.activity_id = $4
                       AND e.objective_id = $5
                )"#,
        )
        .bind(evidence_id)
        .bind(subject_user_id)
        .bind(journey_id)
        .bind(activity_id)
        .bind(objective_id)
        .fetch_one(&self.pool)
        .await
        .map_err(storage_error)?;
        if owned {
            Ok(())
        } else {
            Err(DeepDiveError::SubjectMismatch)
        }
    }

    async fn ensure_generation_run(
        &self,
        subject_user_id: Uuid,
        generation_run_id: Uuid,
    ) -> Result<(), DeepDiveError> {
        let run = crate::generation_postgres::PgGenerationRepository::new(self.pool.clone())
            .get(subject_user_id, generation_run_id)
            .await
            .map_err(DeepDiveError::Generation)?;
        validate_content_run(&run, subject_user_id, "deep_dive.create")
            .map_err(DeepDiveError::Generation)
    }
}

#[async_trait]
impl DeepDiveRepository for PgDeepDiveRepository {
    async fn create(&self, input: CreateDeepDive) -> Result<DeepDive, DeepDiveError> {
        validate_deep_dive(&input)?;
        self.ensure_generation_run(input.subject_user_id, input.generation_run_id)
            .await?;
        self.ensure_owned_evidence(
            input.subject_user_id,
            input.journey_id,
            input.activity_id,
            input.objective_id,
            input.triggering_evidence_id,
        )
        .await?;
        let body = json!({
            "title": input.title,
            "body": input.body,
            "example": input.example,
            "caveats": input.caveats,
        });
        let row = sqlx::query(
            r#"INSERT INTO tb_deep_dives (
                    activity_id, subject_user_id, objective_id,
                    triggering_evidence_id, source_actor_id, generation_run_id, body,
                    source_references, review_status, application_task
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                RETURNING id, content_version, created_at"#,
        )
        .bind(input.activity_id)
        .bind(input.subject_user_id)
        .bind(input.objective_id)
        .bind(input.triggering_evidence_id)
        .bind(input.source_actor_id)
        .bind(input.generation_run_id)
        .bind(body)
        .bind(json!(input.source_references))
        .bind(review_status_value(input.review_status))
        .bind(json!(input.application_task))
        .fetch_one(&self.pool)
        .await
        .map_err(storage_error)?;
        Ok(DeepDive {
            id: row.get("id"),
            input,
            content_version: row.get::<i32, _>("content_version") as u32,
            created_at: row.get("created_at"),
        })
    }

    async fn get(&self, subject_user_id: Uuid, id: Uuid) -> Result<DeepDive, DeepDiveError> {
        let row = sqlx::query(
            "SELECT d.id, d.subject_user_id, d.source_actor_id, d.generation_run_id, a.journey_id, d.activity_id, d.objective_id, d.triggering_evidence_id, d.body, d.source_references, d.review_status, d.application_task, d.content_version, d.created_at FROM tb_deep_dives d JOIN tb_activities a ON a.id = d.activity_id WHERE d.id = $1 AND d.review_status = 'approved'",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?
        .ok_or(DeepDiveError::NotFound)?;
        if row.get::<Uuid, _>("subject_user_id") != subject_user_id {
            return Err(DeepDiveError::SubjectMismatch);
        }
        let body: serde_json::Value = row.get("body");
        Ok(DeepDive {
            id: row.get("id"),
            input: CreateDeepDive {
                subject_user_id: row.get("subject_user_id"),
                source_actor_id: row.get("source_actor_id"),
                generation_run_id: row.get("generation_run_id"),
                journey_id: row.get("journey_id"),
                activity_id: row.get("activity_id"),
                objective_id: row.get("objective_id"),
                triggering_evidence_id: row.get("triggering_evidence_id"),
                title: body
                    .get("title")
                    .and_then(|value| value.as_str())
                    .unwrap_or("Deep dive")
                    .to_string(),
                body: body
                    .get("body")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_string(),
                example: body
                    .get("example")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_string(),
                caveats: serde_json::from_value(body.get("caveats").cloned().unwrap_or_default())
                    .map_err(storage_error)?,
                source_references: serde_json::from_value(row.get("source_references"))
                    .map_err(storage_error)?,
                application_task: row
                    .get::<Option<serde_json::Value>, _>("application_task")
                    .and_then(|value| value.as_str().map(ToString::to_string))
                    .unwrap_or_default(),
                review_status: parse_review_status(row.get("review_status"))?,
            },
            content_version: row.get::<i32, _>("content_version") as u32,
            created_at: row.get("created_at"),
        })
    }

    async fn get_for_activity(
        &self,
        subject_user_id: Uuid,
        activity_id: Uuid,
    ) -> Result<Option<DeepDive>, DeepDiveError> {
        let row = sqlx::query(
            "SELECT d.id, d.subject_user_id, d.source_actor_id, d.generation_run_id, a.journey_id, d.activity_id, d.objective_id, d.triggering_evidence_id, d.body, d.source_references, d.review_status, d.application_task, d.content_version, d.created_at FROM tb_deep_dives d JOIN tb_activities a ON a.id = d.activity_id WHERE d.activity_id = $1 AND d.subject_user_id = $2 AND d.review_status = 'approved'",
        )
        .bind(activity_id)
        .bind(subject_user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?;
        row.map(|row| deep_dive_from_row(&row)).transpose()
    }
}

fn deep_dive_from_row(row: &sqlx::postgres::PgRow) -> Result<DeepDive, DeepDiveError> {
    let body: serde_json::Value = row.get("body");
    Ok(DeepDive {
        id: row.get("id"),
        input: CreateDeepDive {
            subject_user_id: row.get("subject_user_id"),
            source_actor_id: row.get("source_actor_id"),
            generation_run_id: row.get("generation_run_id"),
            journey_id: row.get("journey_id"),
            activity_id: row.get("activity_id"),
            objective_id: row.get("objective_id"),
            triggering_evidence_id: row.get("triggering_evidence_id"),
            title: body
                .get("title")
                .and_then(|value| value.as_str())
                .unwrap_or("Deep dive")
                .to_string(),
            body: body
                .get("body")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string(),
            example: body
                .get("example")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string(),
            caveats: serde_json::from_value(body.get("caveats").cloned().unwrap_or_default())
                .map_err(storage_error)?,
            source_references: serde_json::from_value(row.get("source_references"))
                .map_err(storage_error)?,
            application_task: row
                .get::<Option<serde_json::Value>, _>("application_task")
                .and_then(|value| value.as_str().map(ToString::to_string))
                .unwrap_or_default(),
            review_status: parse_review_status(row.get("review_status"))?,
        },
        content_version: row.get::<i32, _>("content_version") as u32,
        created_at: row.get("created_at"),
    })
}

fn review_status_value(status: ContentReviewStatus) -> &'static str {
    match status {
        ContentReviewStatus::Draft => "draft",
        ContentReviewStatus::Review => "review",
        ContentReviewStatus::Approved => "approved",
        ContentReviewStatus::Rejected | ContentReviewStatus::Retired => "rejected",
    }
}

fn parse_review_status(value: &str) -> Result<ContentReviewStatus, DeepDiveError> {
    match value {
        "draft" => Ok(ContentReviewStatus::Draft),
        "review" => Ok(ContentReviewStatus::Review),
        "approved" => Ok(ContentReviewStatus::Approved),
        "rejected" => Ok(ContentReviewStatus::Rejected),
        _ => Err(DeepDiveError::Storage(format!(
            "unknown deep-dive review status {value}"
        ))),
    }
}

fn storage_error(error: impl std::fmt::Display) -> DeepDiveError {
    DeepDiveError::Storage(error.to_string())
}
