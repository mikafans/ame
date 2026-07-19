//! PostgreSQL projection for the learner timeline.

use crate::domain::timeline::{
    TimelineError, TimelineEvent, TimelineEventKind, TimelineRepository,
};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct PgTimelineRepository {
    pool: PgPool,
}

impl PgTimelineRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TimelineRepository for PgTimelineRepository {
    async fn list_events(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
    ) -> Result<Vec<TimelineEvent>, TimelineError> {
        let rows = sqlx::query(
            r#"
            SELECT event_id, subject_user_id, journey_id, activity_id,
                   event_kind, title, occurred_at
            FROM (
                SELECT ls.id AS event_id, ls.subject_user_id, ls.journey_id,
                       ls.activity_id, 'activity_started' AS event_kind,
                       a.title, ls.started_at AS occurred_at
                FROM tb_learning_sessions ls
                JOIN tb_activities a ON a.id = ls.activity_id
                WHERE ls.subject_user_id = $1 AND ls.journey_id = $2

                UNION ALL

                SELECT ls.id AS event_id, ls.subject_user_id, ls.journey_id,
                       ls.activity_id, 'activity_completed' AS event_kind,
                       a.title, ls.finished_at AS occurred_at
                FROM tb_learning_sessions ls
                JOIN tb_activities a ON a.id = ls.activity_id
                WHERE ls.subject_user_id = $1 AND ls.journey_id = $2
                  AND ls.status = 'finished' AND ls.finished_at IS NOT NULL

                UNION ALL

                SELECT attempt.id AS event_id, attempt.subject_user_id, a.journey_id,
                       attempt.activity_id, 'assessment_submitted' AS event_kind,
                       a.title, attempt.submitted_at AS occurred_at
                FROM tb_attempts attempt
                JOIN tb_activities a ON a.id = attempt.activity_id
                WHERE attempt.subject_user_id = $1 AND a.journey_id = $2
                  AND attempt.status IN ('submitted', 'graded')
                  AND attempt.submitted_at IS NOT NULL

                UNION ALL

                SELECT e.id AS event_id, e.subject_user_id, e.journey_id,
                       e.activity_id, 'evidence_recorded' AS event_kind,
                       a.title, e.created_at AS occurred_at
                FROM tb_mastery_evidence e
                JOIN tb_activities a ON a.id = e.activity_id
                WHERE e.subject_user_id = $1 AND e.journey_id = $2

                UNION ALL

                SELECT d.id AS event_id, d.subject_user_id, a.journey_id,
                       d.activity_id, 'deep_dive_created' AS event_kind,
                       a.title, d.created_at AS occurred_at
                FROM tb_deep_dives d
                JOIN tb_activities a ON a.id = d.activity_id
                WHERE d.subject_user_id = $1 AND a.journey_id = $2

                UNION ALL

                SELECT s.id AS event_id, s.subject_user_id, s.journey_id,
                       s.activity_id, 'streak_recorded' AS event_kind,
                       COALESCE(a.title, 'Learning streak') AS title,
                       s.created_at AS occurred_at
                FROM tb_streak_events s
                LEFT JOIN tb_activities a ON a.id = s.activity_id
                WHERE s.subject_user_id = $1 AND s.journey_id = $2
            ) timeline
            ORDER BY occurred_at ASC, event_id ASC
            "#,
        )
        .bind(subject_user_id)
        .bind(journey_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| TimelineError::Storage(error.to_string()))?;

        rows.into_iter()
            .map(|row| {
                let kind_value: String = row.get("event_kind");
                Ok(TimelineEvent {
                    id: row.get("event_id"),
                    subject_user_id: row.get("subject_user_id"),
                    journey_id: row.get("journey_id"),
                    activity_id: row.get("activity_id"),
                    kind: event_kind(&kind_value)?,
                    title: row.get("title"),
                    occurred_at: row.get("occurred_at"),
                })
            })
            .collect()
    }
}

fn event_kind(value: &str) -> Result<TimelineEventKind, TimelineError> {
    match value {
        "activity_started" => Ok(TimelineEventKind::ActivityStarted),
        "activity_completed" => Ok(TimelineEventKind::ActivityCompleted),
        "assessment_submitted" => Ok(TimelineEventKind::AssessmentSubmitted),
        "evidence_recorded" => Ok(TimelineEventKind::EvidenceRecorded),
        "deep_dive_created" => Ok(TimelineEventKind::DeepDiveCreated),
        "streak_recorded" => Ok(TimelineEventKind::StreakRecorded),
        other => Err(TimelineError::Storage(format!(
            "unknown timeline event kind: {other}"
        ))),
    }
}
