//! PostgreSQL implementation of evidence, mastery, recommendation, and streak behavior.

use crate::{
    domain::progress::{
        EvidenceSource, MasteryEvidence, MasteryEvidenceInput, MasterySnapshot, ProgressError,
        Recommendation, StreakEvent, StreakEventInput, attempt_id_from_event_key,
        qualifying_day_for_attempt, validate_evidence, validate_streak_event,
    },
    progress::ProgressRepository,
};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct PgProgressRepository {
    pool: PgPool,
}

impl PgProgressRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    #[allow(clippy::too_many_arguments)]
    async fn ensure_owned_target(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
        objective_id: Uuid,
        activity_id: Uuid,
        content_version: Option<u32>,
        attempt_id: Option<Uuid>,
        task_submission_id: Option<Uuid>,
        task_score: Option<f32>,
    ) -> Result<(), ProgressError> {
        let owned = sqlx::query_scalar::<_, bool>(
            r#"SELECT EXISTS (
                    SELECT 1
                      FROM tb_learning_journeys j
                      JOIN tb_journey_objectives o
                        ON o.journey_id = j.id
                       AND o.subject_user_id = j.subject_user_id
                      JOIN tb_activities a
                        ON a.journey_id = j.id
                       AND a.subject_user_id = j.subject_user_id
                      JOIN tb_activity_objectives ao
                        ON ao.activity_id = a.id
                       AND ao.objective_id = o.id
                     WHERE j.id = $1
                       AND j.subject_user_id = $2
                       AND o.id = $3
                       AND a.id = $4
                       AND ($5::integer IS NULL OR a.content_version = $5)
                       AND (
                            $6::uuid IS NULL
                            OR EXISTS (
                                SELECT 1
                                  FROM tb_attempts at
                                  JOIN tb_learning_sessions ls
                                    ON ls.id = at.learning_session_id
                                 WHERE at.id = $6
                                   AND at.subject_user_id = $2
                                   AND at.activity_id = $4
                                   AND at.status = 'graded'
                                   AND at.review_status IN ('not_required', 'complete')
                                   AND ls.journey_id = $1
                            )
                       )
                       AND (
                            $7::uuid IS NULL
                            OR EXISTS (
                                SELECT 1
                                  FROM tb_task_submissions ts
                                 WHERE ts.id = $7
                                   AND ts.task_id = $4
                                   AND ts.subject_user_id = $2
                                   AND ts.journey_id = $1
                                   AND ts.content_version = $5
                                   AND ts.status = 'reviewed'
                                   AND ts.review_status = 'complete'
                                   AND ts.score::float4 = $8
                            )
                       )
                )"#,
        )
        .bind(journey_id)
        .bind(subject_user_id)
        .bind(objective_id)
        .bind(activity_id)
        .bind(content_version.map(|version| version as i32))
        .bind(attempt_id)
        .bind(task_submission_id)
        .bind(task_score)
        .fetch_one(&self.pool)
        .await
        .map_err(storage_error)?;
        if owned {
            Ok(())
        } else {
            Err(ProgressError::SubjectMismatch)
        }
    }

    async fn ensure_owned_activity(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
        activity_id: Uuid,
    ) -> Result<(), ProgressError> {
        let owned = sqlx::query_scalar::<_, bool>(
            r#"SELECT EXISTS (
                    SELECT 1
                      FROM tb_learning_journeys j
                      JOIN tb_activities a
                        ON a.journey_id = j.id
                       AND a.subject_user_id = j.subject_user_id
                     WHERE j.id = $1
                       AND j.subject_user_id = $2
                       AND a.id = $3
                )"#,
        )
        .bind(journey_id)
        .bind(subject_user_id)
        .bind(activity_id)
        .fetch_one(&self.pool)
        .await
        .map_err(storage_error)?;
        if owned {
            Ok(())
        } else {
            Err(ProgressError::SubjectMismatch)
        }
    }

    async fn ensure_completed_attempt(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
        activity_id: Uuid,
        attempt_id: Uuid,
    ) -> Result<time::OffsetDateTime, ProgressError> {
        sqlx::query_scalar::<_, time::OffsetDateTime>(
            r#"SELECT at.graded_at
                 FROM tb_attempts at
                 JOIN tb_learning_sessions ls
                   ON ls.id = at.learning_session_id
                WHERE at.id = $1
                  AND at.subject_user_id = $2
                  AND at.activity_id = $3
                  AND ls.journey_id = $4
                  AND at.status = 'graded'
                  AND at.review_status IN ('not_required', 'complete')"#,
        )
        .bind(attempt_id)
        .bind(subject_user_id)
        .bind(activity_id)
        .bind(journey_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?
        .ok_or(ProgressError::SubjectMismatch)
    }

    async fn refresh_snapshot(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
        objective_id: Uuid,
    ) -> Result<MasterySnapshot, ProgressError> {
        let aggregate = sqlx::query(
            r#"SELECT AVG(value)::float4 AS mastery,
                      COUNT(*)::int AS evidence_count,
                      COALESCE(MAX(derivation_version), 1)::int AS derivation_version
                 FROM tb_mastery_evidence
                WHERE subject_user_id = $1
                  AND journey_id = $2
                  AND objective_id = $3"#,
        )
        .bind(subject_user_id)
        .bind(journey_id)
        .bind(objective_id)
        .fetch_one(&self.pool)
        .await
        .map_err(storage_error)?;
        let evidence_count: i32 = aggregate.get("evidence_count");
        if evidence_count == 0 {
            return Err(ProgressError::NotFound);
        }
        let mastery: f32 = aggregate.get("mastery");
        let confidence = (evidence_count as f32 / 3.0).min(1.0);
        let derivation_version: i32 = aggregate.get("derivation_version");
        let row = sqlx::query(
            r#"INSERT INTO tb_mastery_snapshots (
                    subject_user_id, journey_id, objective_id,
                    mastery, confidence, evidence_count, derivation_version
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                ON CONFLICT (subject_user_id, journey_id, objective_id)
                DO UPDATE SET mastery = EXCLUDED.mastery,
                              confidence = EXCLUDED.confidence,
                              evidence_count = EXCLUDED.evidence_count,
                              derivation_version = EXCLUDED.derivation_version,
                              calculated_at = now()
                RETURNING mastery::float4 AS mastery,
                          confidence::float4 AS confidence,
                          evidence_count,
                          calculated_at"#,
        )
        .bind(subject_user_id)
        .bind(journey_id)
        .bind(objective_id)
        .bind(mastery)
        .bind(confidence)
        .bind(evidence_count)
        .bind(derivation_version)
        .fetch_one(&self.pool)
        .await
        .map_err(storage_error)?;
        Ok(MasterySnapshot {
            subject_user_id,
            journey_id,
            objective_id,
            mastery: row.get("mastery"),
            confidence: row.get("confidence"),
            evidence_count: row.get::<i32, _>("evidence_count") as u32,
            calculated_at: row.get("calculated_at"),
        })
    }

    async fn persist_recommendation(
        &self,
        recommendation: &Recommendation,
    ) -> Result<(), ProgressError> {
        let evidence_ids =
            serde_json::to_value(&recommendation.evidence_ids).map_err(storage_error)?;
        let existing = sqlx::query(
            r#"SELECT id
                 FROM tb_recommendations
                WHERE subject_user_id = $1
                  AND journey_id = $2
                  AND objective_id = $3
                  AND activity_id = $4
                  AND reason = $5
                  AND evidence_ids = $6
                  AND status = 'proposed'
                ORDER BY created_at DESC
                LIMIT 1"#,
        )
        .bind(recommendation.subject_user_id)
        .bind(recommendation.journey_id)
        .bind(recommendation.objective_id)
        .bind(recommendation.activity_id)
        .bind(&recommendation.reason)
        .bind(&evidence_ids)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?;
        if existing.is_none() {
            sqlx::query(
                r#"INSERT INTO tb_recommendations (
                        subject_user_id, journey_id, objective_id, activity_id,
                        reason, evidence_ids, status, recommendation_version
                    ) VALUES ($1, $2, $3, $4, $5, $6, 'proposed', 1)"#,
            )
            .bind(recommendation.subject_user_id)
            .bind(recommendation.journey_id)
            .bind(recommendation.objective_id)
            .bind(recommendation.activity_id)
            .bind(&recommendation.reason)
            .bind(evidence_ids)
            .execute(&self.pool)
            .await
            .map_err(storage_error)?;
        }
        Ok(())
    }
}

#[async_trait]
impl ProgressRepository for PgProgressRepository {
    async fn record_evidence(
        &self,
        input: MasteryEvidenceInput,
    ) -> Result<MasteryEvidence, ProgressError> {
        validate_evidence(&input)?;
        self.ensure_owned_target(
            input.subject_user_id,
            input.journey_id,
            input.objective_id,
            input.activity_id,
            Some(input.content_version),
            match input.source {
                EvidenceSource::Assessment { attempt_id } => Some(attempt_id),
                EvidenceSource::Task { .. } => None,
            },
            match input.source {
                EvidenceSource::Assessment { .. } => None,
                EvidenceSource::Task { submission_id } => Some(submission_id),
            },
            match input.source {
                EvidenceSource::Assessment { .. } => None,
                EvidenceSource::Task { .. } => Some(input.value),
            },
        )
        .await?;
        if let EvidenceSource::Task { submission_id } = input.source
            && let Some(row) = sqlx::query(
                "SELECT id, created_at FROM tb_mastery_evidence
                 WHERE task_submission_id = $1 AND objective_id = $2",
            )
            .bind(submission_id)
            .bind(input.objective_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage_error)?
        {
            return Ok(MasteryEvidence {
                id: row.get("id"),
                input,
                created_at: row.get("created_at"),
            });
        }
        let row = sqlx::query(
            r#"INSERT INTO tb_mastery_evidence (
                    subject_user_id, journey_id, objective_id, activity_id,
                    attempt_id, task_submission_id, evidence_type, value, derivation_version,
                    content_version
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                RETURNING id, created_at"#,
        )
        .bind(input.subject_user_id)
        .bind(input.journey_id)
        .bind(input.objective_id)
        .bind(input.activity_id)
        .bind(match input.source {
            EvidenceSource::Assessment { attempt_id } => Some(attempt_id),
            EvidenceSource::Task { .. } => None,
        })
        .bind(match input.source {
            EvidenceSource::Assessment { .. } => None,
            EvidenceSource::Task { submission_id } => Some(submission_id),
        })
        .bind(match input.source {
            EvidenceSource::Assessment { .. } => "attempt",
            EvidenceSource::Task { .. } => "task",
        })
        .bind(input.value)
        .bind(input.derivation_version as i32)
        .bind(input.content_version as i32)
        .fetch_one(&self.pool)
        .await
        .map_err(storage_error)?;
        let evidence = MasteryEvidence {
            id: row.get("id"),
            input,
            created_at: row.get("created_at"),
        };
        self.refresh_snapshot(
            evidence.input.subject_user_id,
            evidence.input.journey_id,
            evidence.input.objective_id,
        )
        .await?;
        Ok(evidence)
    }

    async fn snapshot(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
        objective_id: Uuid,
    ) -> Result<MasterySnapshot, ProgressError> {
        let row = sqlx::query(
            r#"SELECT mastery::float4 AS mastery,
                      confidence::float4 AS confidence,
                      evidence_count,
                      calculated_at
                 FROM tb_mastery_snapshots
                WHERE subject_user_id = $1
                  AND journey_id = $2
                  AND objective_id = $3"#,
        )
        .bind(subject_user_id)
        .bind(journey_id)
        .bind(objective_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?;
        if let Some(row) = row {
            return Ok(MasterySnapshot {
                subject_user_id,
                journey_id,
                objective_id,
                mastery: row.get("mastery"),
                confidence: row.get("confidence"),
                evidence_count: row.get::<i32, _>("evidence_count") as u32,
                calculated_at: row.get("calculated_at"),
            });
        }
        self.refresh_snapshot(subject_user_id, journey_id, objective_id)
            .await
    }

    async fn recommend_weakest(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
        objectives: &[(Uuid, Uuid)],
    ) -> Result<Recommendation, ProgressError> {
        let mut weakest: Option<(f32, i32, Uuid, Uuid)> = None;
        for (objective_id, activity_id) in objectives {
            self.ensure_owned_target(
                subject_user_id,
                journey_id,
                *objective_id,
                *activity_id,
                None,
                None,
                None,
                None,
            )
            .await?;
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
        let recommendation = Recommendation {
            subject_user_id,
            journey_id,
            objective_id,
            activity_id,
            reason: "This objective has the weakest available evidence.".to_string(),
            evidence_ids,
        };
        self.persist_recommendation(&recommendation).await?;
        Ok(recommendation)
    }

    async fn record_streak_event(
        &self,
        input: StreakEventInput,
    ) -> Result<StreakEvent, ProgressError> {
        validate_streak_event(&input)?;
        let attempt_id = attempt_id_from_event_key(&input.qualifying_event_key)?;
        self.ensure_owned_activity(input.subject_user_id, input.journey_id, input.activity_id)
            .await?;
        let graded_at = self
            .ensure_completed_attempt(
                input.subject_user_id,
                input.journey_id,
                input.activity_id,
                attempt_id,
            )
            .await?;
        if qualifying_day_for_attempt(graded_at, &input.learner_timezone)? != input.qualifying_day {
            return Err(ProgressError::InvalidStreakDay);
        }
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
