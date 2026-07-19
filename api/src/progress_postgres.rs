//! PostgreSQL implementation of evidence, mastery, recommendation, and streak behavior.

use crate::{
    domain::progress::{
        MasteryEvidence, MasteryEvidenceInput, MasterySnapshot, ProgressError, Recommendation,
        StreakEvent, StreakEventInput, validate_evidence, validate_streak_event,
    },
    progress::ProgressRepository,
};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone)]
pub struct PgProgressRepository {
    pool: PgPool,
}

impl PgProgressRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProgressRepository for PgProgressRepository {
    async fn record_evidence(
        &self,
        input: MasteryEvidenceInput,
    ) -> Result<MasteryEvidence, ProgressError> {
        validate_evidence(&input)?;
        let row = sqlx::query(
            r#"INSERT INTO tb_mastery_evidence (
                    subject_user_id, journey_id, objective_id, activity_id,
                    attempt_id, evidence_type, value, derivation_version
                )
                VALUES ($1, $2, $3, $4, $5, 'attempt', $6, $7)
                RETURNING id, created_at"#,
        )
        .bind(input.subject_user_id)
        .bind(input.journey_id)
        .bind(input.objective_id)
        .bind(input.activity_id)
        .bind(input.attempt_id)
        .bind(input.value)
        .bind(input.derivation_version as i32)
        .fetch_one(&self.pool)
        .await
        .map_err(storage_error)?;
        Ok(MasteryEvidence {
            id: row.get("id"),
            input,
            created_at: row.get("created_at"),
        })
    }

    async fn snapshot(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
        objective_id: Uuid,
    ) -> Result<MasterySnapshot, ProgressError> {
        let row = sqlx::query(
            "SELECT AVG(value)::float4 AS mastery, COUNT(*)::int AS evidence_count FROM tb_mastery_evidence WHERE subject_user_id = $1 AND journey_id = $2 AND objective_id = $3",
        )
        .bind(subject_user_id)
        .bind(journey_id)
        .bind(objective_id)
        .fetch_one(&self.pool)
        .await
        .map_err(storage_error)?;
        let evidence_count: i32 = row.get("evidence_count");
        if evidence_count == 0 {
            return Err(ProgressError::NotFound);
        }
        let mastery: f32 = row.get("mastery");
        Ok(MasterySnapshot {
            subject_user_id,
            journey_id,
            objective_id,
            mastery,
            confidence: (evidence_count as f32 / 3.0).min(1.0),
            evidence_count: evidence_count as u32,
            calculated_at: OffsetDateTime::now_utc(),
        })
    }

    async fn recommend_weakest(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
        objectives: &[(Uuid, Uuid)],
    ) -> Result<Recommendation, ProgressError> {
        let mut weakest: Option<(f32, i32, Uuid, Uuid)> = None;
        for (objective_id, activity_id) in objectives {
            let candidate = match self
                .snapshot(subject_user_id, journey_id, *objective_id)
                .await
            {
                Ok(snapshot) => (
                    snapshot.mastery,
                    snapshot.evidence_count as i32,
                    *objective_id,
                    *activity_id,
                ),
                Err(ProgressError::NotFound) => (0.0, 0, *objective_id, *activity_id),
                Err(error) => return Err(error),
            };
            if weakest.as_ref().is_none_or(|current| {
                candidate.0 < current.0
                    || (candidate.0 == current.0 && candidate.1 < current.1)
                    || (candidate.0 == current.0
                        && candidate.1 == current.1
                        && candidate.2 < current.2)
            }) {
                weakest = Some(candidate);
            }
        }
        let (_, _, objective_id, activity_id) = weakest.ok_or(ProgressError::NotFound)?;
        let evidence_ids = sqlx::query(
            "SELECT id FROM tb_mastery_evidence WHERE subject_user_id = $1 AND journey_id = $2 AND objective_id = $3 ORDER BY created_at",
        )
        .bind(subject_user_id)
        .bind(journey_id)
        .bind(objective_id)
        .fetch_all(&self.pool)
        .await
        .map_err(storage_error)?
        .into_iter()
        .map(|row| row.get("id"))
        .collect();
        Ok(Recommendation {
            subject_user_id,
            journey_id,
            objective_id,
            activity_id,
            reason: "This objective has the weakest available evidence.".to_string(),
            evidence_ids,
        })
    }

    async fn record_streak_event(
        &self,
        input: StreakEventInput,
    ) -> Result<StreakEvent, ProgressError> {
        validate_streak_event(&input)?;
        let row = sqlx::query(
            r#"INSERT INTO tb_streak_events (
                    subject_user_id, journey_id, activity_id, qualifying_event_key,
                    learner_timezone, qualifying_day
                ) VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (subject_user_id, qualifying_event_key) DO NOTHING
                RETURNING id, created_at"#,
        )
        .bind(input.subject_user_id)
        .bind(input.journey_id)
        .bind(input.activity_id)
        .bind(&input.qualifying_event_key)
        .bind(&input.learner_timezone)
        .bind(input.qualifying_day)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?;
        if let Some(row) = row {
            return Ok(StreakEvent {
                id: row.get("id"),
                input,
                created_at: row.get("created_at"),
            });
        }
        let existing = sqlx::query(
            "SELECT id, subject_user_id, journey_id, activity_id, qualifying_event_key, learner_timezone, qualifying_day, created_at FROM tb_streak_events WHERE subject_user_id = $1 AND qualifying_event_key = $2",
        )
        .bind(input.subject_user_id)
        .bind(&input.qualifying_event_key)
        .fetch_one(&self.pool)
        .await
        .map_err(storage_error)?;
        let existing_input = StreakEventInput {
            subject_user_id: existing.get("subject_user_id"),
            journey_id: existing.get("journey_id"),
            activity_id: existing.get("activity_id"),
            qualifying_event_key: existing.get("qualifying_event_key"),
            learner_timezone: existing.get("learner_timezone"),
            qualifying_day: existing.get("qualifying_day"),
        };
        if existing_input == input {
            Ok(StreakEvent {
                id: existing.get("id"),
                input: existing_input,
                created_at: existing.get("created_at"),
            })
        } else {
            Err(ProgressError::DuplicateStreakEvent)
        }
    }

    async fn list_streak_events(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
    ) -> Result<Vec<StreakEvent>, ProgressError> {
        Ok(sqlx::query(
            "SELECT id, subject_user_id, journey_id, activity_id, qualifying_event_key, learner_timezone, qualifying_day, created_at FROM tb_streak_events WHERE subject_user_id = $1 AND journey_id = $2 ORDER BY qualifying_day, created_at",
        )
        .bind(subject_user_id)
        .bind(journey_id)
        .fetch_all(&self.pool)
        .await
        .map_err(storage_error)?
        .into_iter()
        .map(|row| StreakEvent {
            id: row.get("id"),
            input: StreakEventInput {
                subject_user_id: row.get("subject_user_id"),
                journey_id: row.get("journey_id"),
                activity_id: row.get("activity_id"),
                qualifying_event_key: row.get("qualifying_event_key"),
                learner_timezone: row.get("learner_timezone"),
                qualifying_day: row.get("qualifying_day"),
            },
            created_at: row.get("created_at"),
        })
        .collect())
    }
}

fn storage_error(error: impl std::fmt::Display) -> ProgressError {
    ProgressError::Storage(error.to_string())
}
