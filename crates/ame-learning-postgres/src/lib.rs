//! PostgreSQL implementation of the learning repository contract.

use ame_learning_domain::{
    ActivityKind, ActivityPublicationStatus, ActivityStatus, AuthorActivityContent,
    AuthorActivityRubric, CreateActivity, CreateChapter, CreateGoal, CreateJourney,
    CreateLearningSession, CreateObjective, FinishLearningSession, GoalStatus, JourneyStatus,
    LearningActivity, LearningChapter, LearningGoal, LearningJourney, LearningObjective,
    LearningRepository, LearningRepositoryError, LearningSession, LearningSessionStatus,
    ObjectiveStatus, validate_activity, validate_activity_content, validate_activity_rubric,
    validate_chapter, validate_goal, validate_journey, validate_objective,
};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct PgLearningRepository {
    pool: PgPool,
}

impl PgLearningRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn find_goal_by_id(
        &self,
        goal_id: Uuid,
    ) -> Result<Option<LearningGoal>, LearningRepositoryError> {
        sqlx::query(
            r#"
            SELECT id, subject_user_id, source_actor_id, template_version_id,
                   raw_intent, normalized_statement, status, idempotency_key, created_at
            FROM tb_learning_goals
            WHERE id = $1
            "#,
        )
        .bind(goal_id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(goal_from_row))
        .map_err(storage_error)
    }

    async fn find_journey_by_id(
        &self,
        journey_id: Uuid,
    ) -> Result<Option<LearningJourney>, LearningRepositoryError> {
        sqlx::query(
            r#"
            SELECT id, goal_id, subject_user_id, source_actor_id, promise, status, created_at
            FROM tb_learning_journeys
            WHERE id = $1
            "#,
        )
        .bind(journey_id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(journey_from_row))
        .map_err(storage_error)
    }

    async fn find_journey_by_goal(
        &self,
        goal_id: Uuid,
    ) -> Result<Option<LearningJourney>, LearningRepositoryError> {
        sqlx::query(
            r#"
            SELECT id, goal_id, subject_user_id, source_actor_id, promise, status, created_at
            FROM tb_learning_journeys
            WHERE goal_id = $1
            "#,
        )
        .bind(goal_id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(journey_from_row))
        .map_err(storage_error)
    }
}

#[async_trait]
impl LearningRepository for PgLearningRepository {
    async fn create_goal(
        &self,
        input: CreateGoal,
    ) -> Result<LearningGoal, LearningRepositoryError> {
        validate_goal(&input)?;

        let inserted = sqlx::query(
            r#"
            INSERT INTO tb_learning_goals (
                subject_user_id, source_actor_id, template_version_id,
                raw_intent, normalized_statement, status, idempotency_key
            )
            VALUES ($1, $2, $3, $4, $5, 'proposed', $6)
            ON CONFLICT (subject_user_id, idempotency_key)
                WHERE idempotency_key IS NOT NULL
                DO NOTHING
            RETURNING id, subject_user_id, source_actor_id, template_version_id,
                      raw_intent, normalized_statement, status, idempotency_key, created_at
            "#,
        )
        .bind(input.subject_user_id)
        .bind(input.source_actor_id)
        .bind(input.template_version_id)
        .bind(&input.raw_intent)
        .bind(&input.normalized_statement)
        .bind(&input.idempotency_key)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?
        .map(goal_from_row);

        if let Some(goal) = inserted {
            return Ok(goal);
        }

        let Some(key) = input.idempotency_key else {
            return Err(LearningRepositoryError::Storage(
                "goal insert returned no row without an idempotency key".to_string(),
            ));
        };
        let existing = sqlx::query(
            r#"
            SELECT id, subject_user_id, source_actor_id, template_version_id,
                   raw_intent, normalized_statement, status, idempotency_key, created_at
            FROM tb_learning_goals
            WHERE subject_user_id = $1 AND idempotency_key = $2
            "#,
        )
        .bind(input.subject_user_id)
        .bind(key)
        .fetch_one(&self.pool)
        .await
        .map_err(storage_error)
        .map(goal_from_row)?;

        if existing.raw_intent == input.raw_intent
            && existing.normalized_statement == input.normalized_statement
            && existing.source_actor_id == input.source_actor_id
            && existing.template_version_id == input.template_version_id
        {
            Ok(existing)
        } else {
            Err(LearningRepositoryError::IdempotencyConflict)
        }
    }

    async fn get_goal(
        &self,
        subject_user_id: Uuid,
        goal_id: Uuid,
    ) -> Result<LearningGoal, LearningRepositoryError> {
        match self.find_goal_by_id(goal_id).await? {
            Some(goal) if goal.subject_user_id == subject_user_id => Ok(goal),
            Some(_) => Err(LearningRepositoryError::SubjectMismatch),
            None => Err(LearningRepositoryError::NotFound { resource: "goal" }),
        }
    }

    async fn ensure_journey(
        &self,
        input: CreateJourney,
    ) -> Result<LearningJourney, LearningRepositoryError> {
        validate_journey(&input)?;
        let goal = self.get_goal(input.subject_user_id, input.goal_id).await?;

        let inserted = sqlx::query(
            r#"
            INSERT INTO tb_learning_journeys (
                goal_id, subject_user_id, source_actor_id, promise, status
            )
            VALUES ($1, $2, $3, $4, 'onboarding')
            ON CONFLICT (goal_id)
                DO NOTHING
            RETURNING id, goal_id, subject_user_id, source_actor_id, promise, status, created_at
            "#,
        )
        .bind(goal.id)
        .bind(input.subject_user_id)
        .bind(input.source_actor_id)
        .bind(&input.promise)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?
        .map(journey_from_row);

        if let Some(journey) = inserted {
            return Ok(journey);
        }

        self.find_journey_by_goal(goal.id)
            .await?
            .ok_or(LearningRepositoryError::NotFound {
                resource: "journey",
            })
    }

    async fn get_journey(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
    ) -> Result<LearningJourney, LearningRepositoryError> {
        match self.find_journey_by_id(journey_id).await? {
            Some(journey) if journey.subject_user_id == subject_user_id => Ok(journey),
            Some(_) => Err(LearningRepositoryError::SubjectMismatch),
            None => Err(LearningRepositoryError::NotFound {
                resource: "journey",
            }),
        }
    }

    async fn list_journeys(
        &self,
        subject_user_id: Uuid,
    ) -> Result<Vec<LearningJourney>, LearningRepositoryError> {
        let rows = sqlx::query(
            r#"
            SELECT id, goal_id, subject_user_id, source_actor_id, promise, status, created_at
            FROM tb_learning_journeys
            WHERE subject_user_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(subject_user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(storage_error)?;
        Ok(rows.into_iter().map(journey_from_row).collect())
    }

    async fn set_journey_status(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
        status: JourneyStatus,
    ) -> Result<LearningJourney, LearningRepositoryError> {
        let row = sqlx::query(
            r#"
            UPDATE tb_learning_journeys
            SET status = $3, updated_at = now()
            WHERE id = $1 AND subject_user_id = $2
            RETURNING id, goal_id, subject_user_id, source_actor_id, promise, status, created_at
            "#,
        )
        .bind(journey_id)
        .bind(subject_user_id)
        .bind(journey_status_value(status))
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?;

        match row {
            Some(row) => Ok(journey_from_row(row)),
            None => match self.find_journey_by_id(journey_id).await? {
                Some(_) => Err(LearningRepositoryError::SubjectMismatch),
                None => Err(LearningRepositoryError::NotFound {
                    resource: "journey",
                }),
            },
        }
    }

    async fn create_objective(
        &self,
        input: CreateObjective,
    ) -> Result<LearningObjective, LearningRepositoryError> {
        validate_objective(&input)?;
        let journey = self
            .get_journey(input.subject_user_id, input.journey_id)
            .await?;
        let row = sqlx::query(
            r#"
            INSERT INTO tb_journey_objectives (
                journey_id, subject_user_id, verb, statement, success_criteria, order_index
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, journey_id, subject_user_id, verb, statement,
                      success_criteria, order_index, status, created_at
            "#,
        )
        .bind(journey.id)
        .bind(input.subject_user_id)
        .bind(&input.verb)
        .bind(&input.statement)
        .bind(&input.success_criteria)
        .bind(input.order_index)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| {
            if let sqlx::Error::Database(database) = &error
                && database.constraint() == Some("tb_journey_objectives_journey_id_order_index_key")
            {
                return LearningRepositoryError::OrderConflict {
                    resource: "objective",
                };
            }
            storage_error(error)
        })?;
        Ok(objective_from_row(row))
    }

    async fn create_chapter(
        &self,
        input: CreateChapter,
    ) -> Result<LearningChapter, LearningRepositoryError> {
        validate_chapter(&input)?;
        let journey = self
            .get_journey(input.subject_user_id, input.journey_id)
            .await?;
        let row = sqlx::query(
            r#"
            INSERT INTO tb_journey_chapters (
                journey_id, subject_user_id, title, summary, order_index
            )
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, journey_id, subject_user_id, title, summary, order_index, created_at
            "#,
        )
        .bind(journey.id)
        .bind(input.subject_user_id)
        .bind(&input.title)
        .bind(&input.summary)
        .bind(input.order_index)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| {
            if let sqlx::Error::Database(database) = &error
                && database.constraint() == Some("tb_journey_chapters_journey_id_order_index_key")
            {
                return LearningRepositoryError::OrderConflict {
                    resource: "chapter",
                };
            }
            storage_error(error)
        })?;
        Ok(chapter_from_row(row))
    }

    async fn list_chapters(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
    ) -> Result<Vec<LearningChapter>, LearningRepositoryError> {
        self.get_journey(subject_user_id, journey_id).await?;
        let rows = sqlx::query(
            r#"
            SELECT id, journey_id, subject_user_id, title, summary, order_index, created_at
            FROM tb_journey_chapters
            WHERE journey_id = $1 AND subject_user_id = $2
            ORDER BY order_index ASC
            "#,
        )
        .bind(journey_id)
        .bind(subject_user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(storage_error)?;
        Ok(rows.into_iter().map(chapter_from_row).collect())
    }

    async fn list_objectives(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
    ) -> Result<Vec<LearningObjective>, LearningRepositoryError> {
        self.get_journey(subject_user_id, journey_id).await?;
        let rows = sqlx::query(
            r#"
            SELECT id, journey_id, subject_user_id, verb, statement,
                   success_criteria, order_index, status, created_at
            FROM tb_journey_objectives
            WHERE journey_id = $1 AND subject_user_id = $2
            ORDER BY order_index ASC
            "#,
        )
        .bind(journey_id)
        .bind(subject_user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(storage_error)?;
        Ok(rows.into_iter().map(objective_from_row).collect())
    }

    async fn create_activity(
        &self,
        input: CreateActivity,
    ) -> Result<LearningActivity, LearningRepositoryError> {
        validate_activity(&input)?;
        let journey = self
            .get_journey(input.subject_user_id, input.journey_id)
            .await?;
        if let Some(chapter_id) = input.chapter_id {
            let chapter = sqlx::query(
                "SELECT journey_id, subject_user_id FROM tb_journey_chapters WHERE id = $1",
            )
            .bind(chapter_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage_error)?
            .ok_or(LearningRepositoryError::NotFound {
                resource: "chapter",
            })?;
            if chapter.get::<Uuid, _>("journey_id") != journey.id
                || chapter.get::<Uuid, _>("subject_user_id") != input.subject_user_id
            {
                return Err(LearningRepositoryError::SubjectMismatch);
            }
        }
        for objective_id in &input.objective_ids {
            let row = sqlx::query(
                "SELECT 1 FROM tb_journey_objectives WHERE id = $1 AND journey_id = $2 AND subject_user_id = $3",
            )
            .bind(objective_id)
            .bind(journey.id)
            .bind(input.subject_user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage_error)?;
            if row.is_none() {
                return Err(LearningRepositoryError::SubjectMismatch);
            }
        }

        let mut transaction = self.pool.begin().await.map_err(storage_error)?;
        let row = sqlx::query(
            r#"
            INSERT INTO tb_activities (
                journey_id, subject_user_id, source_actor_id,
                chapter_id, kind, title, order_index, payload_schema_version,
                content_version, publication_status, payload, status, rubric
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING id, journey_id, subject_user_id, source_actor_id,
                      chapter_id, kind, title, order_index, payload_schema_version,
                      content_version, publication_status, payload,
                      status, rubric, created_at, updated_at
            "#,
        )
        .bind(journey.id)
        .bind(input.subject_user_id)
        .bind(input.source_actor_id)
        .bind(input.chapter_id)
        .bind(activity_kind_value(input.kind))
        .bind(&input.title)
        .bind(input.order_index)
        .bind(input.payload_schema_version)
        .bind(input.content_version)
        .bind(publication_status_value(input.publication_status))
        .bind(&input.payload)
        .bind(activity_status_value(input.status))
        .bind(&input.rubric)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|error| {
            if let sqlx::Error::Database(database) = &error
                && matches!(
                    database.constraint(),
                    Some("tb_activities_chapter_order")
                        | Some("tb_activities_journey_order_ungrouped")
                )
            {
                return LearningRepositoryError::OrderConflict {
                    resource: "activity",
                };
            }
            storage_error(error)
        })?;
        let activity_id: Uuid = row.get("id");
        for objective_id in &input.objective_ids {
            sqlx::query(
                "INSERT INTO tb_activity_objectives (activity_id, objective_id) VALUES ($1, $2)",
            )
            .bind(activity_id)
            .bind(objective_id)
            .execute(&mut *transaction)
            .await
            .map_err(storage_error)?;
        }
        transaction.commit().await.map_err(storage_error)?;
        Ok(activity_from_row(row, input.objective_ids))
    }

    async fn list_activities(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
    ) -> Result<Vec<LearningActivity>, LearningRepositoryError> {
        self.get_journey(subject_user_id, journey_id).await?;
        let rows = sqlx::query(
            r#"
            SELECT a.id, a.journey_id, a.subject_user_id, a.source_actor_id,
                   a.chapter_id, a.kind, a.title, a.order_index,
                   a.payload_schema_version, a.content_version,
                   a.publication_status, a.payload, a.status, a.rubric,
                   a.created_at, a.updated_at,
                   COALESCE(array_agg(ao.objective_id) FILTER (WHERE ao.objective_id IS NOT NULL), ARRAY[]::uuid[]) AS objective_ids
            FROM tb_activities a
            LEFT JOIN tb_activity_objectives ao ON ao.activity_id = a.id
            WHERE a.journey_id = $1 AND a.subject_user_id = $2
            GROUP BY a.id
            ORDER BY COALESCE((SELECT c.order_index FROM tb_journey_chapters c WHERE c.id = a.chapter_id), 2147483647), a.order_index ASC
            "#,
        )
        .bind(journey_id)
        .bind(subject_user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(storage_error)?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let objective_ids: Vec<Uuid> = row.get("objective_ids");
                activity_from_row(row, objective_ids)
            })
            .collect())
    }

    async fn author_activity_content(
        &self,
        input: AuthorActivityContent,
    ) -> Result<LearningActivity, LearningRepositoryError> {
        let row = sqlx::query(
            "SELECT journey_id, subject_user_id, kind, status FROM tb_activities WHERE id = $1",
        )
        .bind(input.activity_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?
        .ok_or(LearningRepositoryError::NotFound {
            resource: "activity",
        })?;
        let subject_user_id: Uuid = row.get("subject_user_id");
        if subject_user_id != input.subject_user_id {
            return Err(LearningRepositoryError::SubjectMismatch);
        }
        let kind = activity_kind(row.get::<String, _>("kind").as_str());
        if !matches!(kind, ActivityKind::Explanation | ActivityKind::Example) {
            return Err(LearningRepositoryError::InvalidActivityContent);
        }
        validate_activity_content(kind, &input)?;
        if activity_status(row.get::<String, _>("status").as_str()) == ActivityStatus::Completed {
            return Err(LearningRepositoryError::ActivityContentCompleted);
        }
        let journey_id: Uuid = row.get("journey_id");
        let mut activity = self
            .list_activities(input.subject_user_id, journey_id)
            .await?
            .into_iter()
            .find(|activity| activity.id == input.activity_id)
            .ok_or(LearningRepositoryError::NotFound {
                resource: "activity",
            })?;
        let payload = activity
            .payload
            .as_object_mut()
            .ok_or(LearningRepositoryError::InvalidActivityContent)?;
        payload.insert("content".into(), input.content);
        payload.insert(
            "contentProvenance".into(),
            serde_json::json!({
                "generationRunId": input.generation_run_id,
                "reviewStatus": input.review_status,
                "sourceReferences": input.source_references,
            }),
        );
        let updated_at = sqlx::query(
            "UPDATE tb_activities SET payload = $1, updated_at = now() WHERE id = $2 AND subject_user_id = $3 AND status <> 'completed' RETURNING updated_at",
        )
        .bind(&activity.payload)
        .bind(input.activity_id)
        .bind(input.subject_user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?
        .ok_or(LearningRepositoryError::ActivityContentCompleted)?
        .get("updated_at");
        activity.updated_at = updated_at;
        Ok(activity)
    }

    async fn author_activity_rubric(
        &self,
        input: AuthorActivityRubric,
    ) -> Result<LearningActivity, LearningRepositoryError> {
        validate_activity_rubric(&input)?;
        let row = sqlx::query(
            "SELECT subject_user_id, journey_id, status FROM tb_activities WHERE id = $1",
        )
        .bind(input.activity_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?
        .ok_or(LearningRepositoryError::NotFound {
            resource: "activity",
        })?;
        if row.get::<Uuid, _>("subject_user_id") != input.subject_user_id {
            return Err(LearningRepositoryError::SubjectMismatch);
        }
        if activity_status(row.get::<String, _>("status").as_str()) == ActivityStatus::Completed {
            return Err(LearningRepositoryError::ActivityContentCompleted);
        }
        let journey_id: Uuid = row.get("journey_id");
        sqlx::query(
            "UPDATE tb_activities SET rubric = $1, updated_at = now() WHERE id = $2 AND subject_user_id = $3 AND status <> 'completed'",
        )
        .bind(&input.rubric)
        .bind(input.activity_id)
        .bind(input.subject_user_id)
        .execute(&self.pool)
        .await
        .map_err(storage_error)?;
        self.list_activities(input.subject_user_id, journey_id)
            .await?
            .into_iter()
            .find(|activity| activity.id == input.activity_id)
            .ok_or(LearningRepositoryError::NotFound {
                resource: "activity",
            })
    }

    async fn start_learning_session(
        &self,
        input: CreateLearningSession,
    ) -> Result<LearningSession, LearningRepositoryError> {
        let journey = self
            .get_journey(input.subject_user_id, input.journey_id)
            .await?;
        let activity = sqlx::query(
            "SELECT journey_id, subject_user_id, status FROM tb_activities WHERE id = $1",
        )
        .bind(input.activity_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?
        .ok_or(LearningRepositoryError::NotFound {
            resource: "activity",
        })?;
        let activity_journey_id: Uuid = activity.get("journey_id");
        let activity_subject_id: Uuid = activity.get("subject_user_id");
        if activity_journey_id != journey.id || activity_subject_id != input.subject_user_id {
            return Err(LearningRepositoryError::SubjectMismatch);
        }
        if activity.get::<String, _>("status") != "ready" {
            return Err(LearningRepositoryError::ActivityNotReady);
        }
        sqlx::query(
            "UPDATE tb_learning_journeys SET status = 'active', updated_at = now() WHERE id = $1 AND subject_user_id = $2",
        )
        .bind(journey.id)
        .bind(input.subject_user_id)
        .execute(&self.pool)
        .await
        .map_err(storage_error)?;
        sqlx::query(
            "UPDATE tb_learning_goals SET status = 'active', updated_at = now() WHERE id = $1 AND subject_user_id = $2",
        )
        .bind(journey.goal_id)
        .bind(input.subject_user_id)
        .execute(&self.pool)
        .await
        .map_err(storage_error)?;

        if let Some(row) = sqlx::query(
            r#"
            SELECT id, journey_id, activity_id, subject_user_id, actor_identity_id,
                   status, question_plan, result, started_at, finished_at
            FROM tb_learning_sessions
            WHERE activity_id = $1 AND subject_user_id = $2 AND status = 'in_progress'
            ORDER BY started_at DESC
            LIMIT 1
            "#,
        )
        .bind(input.activity_id)
        .bind(input.subject_user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?
        {
            return Ok(learning_session_from_row(row));
        }

        let row = sqlx::query(
            r#"
            INSERT INTO tb_learning_sessions (
                journey_id, activity_id, subject_user_id, actor_identity_id, question_plan
            )
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, journey_id, activity_id, subject_user_id, actor_identity_id,
                      status, question_plan, result, started_at, finished_at
            "#,
        )
        .bind(journey.id)
        .bind(input.activity_id)
        .bind(input.subject_user_id)
        .bind(input.actor_identity_id)
        .bind(input.question_plan)
        .fetch_one(&self.pool)
        .await
        .map_err(storage_error)?;
        Ok(learning_session_from_row(row))
    }

    async fn get_learning_session(
        &self,
        subject_user_id: Uuid,
        session_id: Uuid,
    ) -> Result<LearningSession, LearningRepositoryError> {
        let row = sqlx::query(
            r#"
            SELECT id, journey_id, activity_id, subject_user_id, actor_identity_id,
                   status, question_plan, result, started_at, finished_at
            FROM tb_learning_sessions
            WHERE id = $1
            "#,
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?;
        match row {
            Some(row) if row.get::<Uuid, _>("subject_user_id") == subject_user_id => {
                Ok(learning_session_from_row(row))
            }
            Some(_) => Err(LearningRepositoryError::SubjectMismatch),
            None => Err(LearningRepositoryError::NotFound {
                resource: "learning session",
            }),
        }
    }

    async fn latest_finished_learning_session(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
    ) -> Result<Option<LearningSession>, LearningRepositoryError> {
        let journey = self.get_journey(subject_user_id, journey_id).await?;
        sqlx::query(
            r#"
            SELECT id, journey_id, activity_id, subject_user_id, actor_identity_id,
                   status, question_plan, result, started_at, finished_at
            FROM tb_learning_sessions
            WHERE journey_id = $1 AND subject_user_id = $2 AND status = 'finished'
            ORDER BY finished_at DESC
            LIMIT 1
            "#,
        )
        .bind(journey.id)
        .bind(subject_user_id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(learning_session_from_row))
        .map_err(storage_error)
    }

    async fn finish_learning_session(
        &self,
        input: FinishLearningSession,
    ) -> Result<LearningSession, LearningRepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(storage_error)?;
        let row = sqlx::query(
            r#"
            UPDATE tb_learning_sessions
            SET status = $3, result = $4, finished_at = now()
            WHERE id = $1 AND subject_user_id = $2 AND status = 'in_progress'
            RETURNING id, journey_id, activity_id, subject_user_id, actor_identity_id,
                      status, question_plan, result, started_at, finished_at
            "#,
        )
        .bind(input.session_id)
        .bind(input.subject_user_id)
        .bind(if input.completed {
            "finished"
        } else {
            "abandoned"
        })
        .bind(&input.result)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(storage_error)?;
        let Some(row) = row else {
            let current = sqlx::query(
                r#"SELECT id, journey_id, activity_id, subject_user_id, actor_identity_id,
                          status, question_plan, result, started_at, finished_at
                   FROM tb_learning_sessions
                   WHERE id = $1"#,
            )
            .bind(input.session_id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(storage_error)?;
            return match current {
                Some(current)
                    if current.get::<Uuid, _>("subject_user_id") != input.subject_user_id =>
                {
                    Err(LearningRepositoryError::SubjectMismatch)
                }
                Some(current) => {
                    let session = learning_session_from_row(current);
                    match session.result.as_ref() {
                        Some(result) if *result == input.result => Ok(session),
                        Some(_) => Err(LearningRepositoryError::LearningSessionResultConflict),
                        None => Err(LearningRepositoryError::LearningSessionFinished),
                    }
                }
                None => Err(LearningRepositoryError::NotFound {
                    resource: "learning session",
                }),
            };
        };
        let session = learning_session_from_row(row);
        if !input.completed {
            transaction.commit().await.map_err(storage_error)?;
            return Ok(session);
        }
        sqlx::query(
            "UPDATE tb_activities SET status = 'completed', updated_at = now() WHERE id = $1",
        )
        .bind(session.activity_id)
        .execute(&mut *transaction)
        .await
        .map_err(storage_error)?;
        sqlx::query(
            r#"
            UPDATE tb_activities
            SET status = 'ready', updated_at = now()
            WHERE id = (
                SELECT next_activity.id
                FROM tb_activities next_activity
                JOIN tb_activities completed_activity
                  ON completed_activity.id = $1
                WHERE next_activity.journey_id = completed_activity.journey_id
                  AND next_activity.order_index > completed_activity.order_index
                  AND next_activity.status = 'proposed'
                ORDER BY next_activity.order_index ASC
                LIMIT 1
            )
            "#,
        )
        .bind(session.activity_id)
        .execute(&mut *transaction)
        .await
        .map_err(storage_error)?;
        transaction.commit().await.map_err(storage_error)?;
        Ok(session)
    }
}

fn storage_error(error: sqlx::Error) -> LearningRepositoryError {
    LearningRepositoryError::storage(error)
}

fn goal_from_row(row: sqlx::postgres::PgRow) -> LearningGoal {
    LearningGoal {
        id: row.get("id"),
        subject_user_id: row.get("subject_user_id"),
        source_actor_id: row.get("source_actor_id"),
        template_version_id: row.get("template_version_id"),
        raw_intent: row.get("raw_intent"),
        normalized_statement: row.get("normalized_statement"),
        status: goal_status(row.get::<String, _>("status").as_str()),
        idempotency_key: row.get("idempotency_key"),
        created_at: row.get("created_at"),
    }
}

fn journey_from_row(row: sqlx::postgres::PgRow) -> LearningJourney {
    LearningJourney {
        id: row.get("id"),
        goal_id: row.get("goal_id"),
        subject_user_id: row.get("subject_user_id"),
        source_actor_id: row.get("source_actor_id"),
        promise: row.get("promise"),
        status: journey_status(row.get::<String, _>("status").as_str()),
        created_at: row.get("created_at"),
    }
}

fn chapter_from_row(row: sqlx::postgres::PgRow) -> LearningChapter {
    LearningChapter {
        id: row.get("id"),
        journey_id: row.get("journey_id"),
        subject_user_id: row.get("subject_user_id"),
        title: row.get("title"),
        summary: row.get("summary"),
        order_index: row.get("order_index"),
        created_at: row.get("created_at"),
    }
}

fn goal_status(value: &str) -> GoalStatus {
    match value {
        "active" => GoalStatus::Active,
        "paused" => GoalStatus::Paused,
        "completed" => GoalStatus::Completed,
        "failed" => GoalStatus::Failed,
        _ => GoalStatus::Proposed,
    }
}

fn journey_status(value: &str) -> JourneyStatus {
    match value {
        "active" => JourneyStatus::Active,
        "paused" => JourneyStatus::Paused,
        "completed" => JourneyStatus::Completed,
        "failed" => JourneyStatus::Failed,
        _ => JourneyStatus::Onboarding,
    }
}

fn journey_status_value(status: JourneyStatus) -> &'static str {
    match status {
        JourneyStatus::Onboarding => "onboarding",
        JourneyStatus::Active => "active",
        JourneyStatus::Paused => "paused",
        JourneyStatus::Completed => "completed",
        JourneyStatus::Failed => "failed",
    }
}

fn objective_from_row(row: sqlx::postgres::PgRow) -> LearningObjective {
    LearningObjective {
        id: row.get("id"),
        journey_id: row.get("journey_id"),
        subject_user_id: row.get("subject_user_id"),
        verb: row.get("verb"),
        statement: row.get("statement"),
        success_criteria: row.get("success_criteria"),
        order_index: row.get("order_index"),
        status: objective_status(row.get::<String, _>("status").as_str()),
        created_at: row.get("created_at"),
    }
}

fn activity_from_row(row: sqlx::postgres::PgRow, objective_ids: Vec<Uuid>) -> LearningActivity {
    LearningActivity {
        id: row.get("id"),
        journey_id: row.get("journey_id"),
        subject_user_id: row.get("subject_user_id"),
        source_actor_id: row.get("source_actor_id"),
        chapter_id: row.get("chapter_id"),
        kind: activity_kind(row.get::<String, _>("kind").as_str()),
        title: row.get("title"),
        order_index: row.get("order_index"),
        payload_schema_version: row.get("payload_schema_version"),
        content_version: row.get("content_version"),
        publication_status: activity_publication_status(
            row.get::<String, _>("publication_status").as_str(),
        ),
        payload: row.get("payload"),
        objective_ids,
        status: activity_status(row.get::<String, _>("status").as_str()),
        rubric: row.get("rubric"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn objective_status(value: &str) -> ObjectiveStatus {
    match value {
        "paused" => ObjectiveStatus::Paused,
        "completed" => ObjectiveStatus::Completed,
        _ => ObjectiveStatus::Active,
    }
}

fn activity_kind(value: &str) -> ActivityKind {
    match value {
        "example" => ActivityKind::Example,
        "diagnostic" => ActivityKind::Diagnostic,
        "practice" => ActivityKind::Practice,
        "feedback" => ActivityKind::Feedback,
        "application" => ActivityKind::Application,
        "reflection" => ActivityKind::Reflection,
        "milestone" => ActivityKind::Milestone,
        "timed_practice" => ActivityKind::TimedPractice,
        "recommendation" => ActivityKind::Recommendation,
        _ => ActivityKind::Explanation,
    }
}

fn activity_kind_value(kind: ActivityKind) -> &'static str {
    match kind {
        ActivityKind::Explanation => "explanation",
        ActivityKind::Example => "example",
        ActivityKind::Diagnostic => "diagnostic",
        ActivityKind::Practice => "practice",
        ActivityKind::Feedback => "feedback",
        ActivityKind::Application => "application",
        ActivityKind::Reflection => "reflection",
        ActivityKind::Milestone => "milestone",
        ActivityKind::TimedPractice => "timed_practice",
        ActivityKind::Recommendation => "recommendation",
    }
}

fn activity_status(value: &str) -> ActivityStatus {
    match value {
        "ready" => ActivityStatus::Ready,
        "in_progress" => ActivityStatus::InProgress,
        "completed" => ActivityStatus::Completed,
        "failed" => ActivityStatus::Failed,
        _ => ActivityStatus::Proposed,
    }
}

fn activity_status_value(status: ActivityStatus) -> &'static str {
    match status {
        ActivityStatus::Proposed => "proposed",
        ActivityStatus::Ready => "ready",
        ActivityStatus::InProgress => "in_progress",
        ActivityStatus::Completed => "completed",
        ActivityStatus::Failed => "failed",
    }
}

fn activity_publication_status(value: &str) -> ActivityPublicationStatus {
    match value {
        "review" => ActivityPublicationStatus::Review,
        "published" => ActivityPublicationStatus::Published,
        "retired" => ActivityPublicationStatus::Retired,
        _ => ActivityPublicationStatus::Draft,
    }
}

fn publication_status_value(status: ActivityPublicationStatus) -> &'static str {
    match status {
        ActivityPublicationStatus::Draft => "draft",
        ActivityPublicationStatus::Review => "review",
        ActivityPublicationStatus::Published => "published",
        ActivityPublicationStatus::Retired => "retired",
    }
}

fn learning_session_from_row(row: sqlx::postgres::PgRow) -> LearningSession {
    LearningSession {
        id: row.get("id"),
        journey_id: row.get("journey_id"),
        activity_id: row.get("activity_id"),
        subject_user_id: row.get("subject_user_id"),
        actor_identity_id: row.get("actor_identity_id"),
        status: learning_session_status(row.get::<String, _>("status").as_str()),
        question_plan: row.get("question_plan"),
        result: row.get("result"),
        started_at: row.get("started_at"),
        finished_at: row.get("finished_at"),
    }
}

fn learning_session_status(value: &str) -> LearningSessionStatus {
    match value {
        "finished" => LearningSessionStatus::Finished,
        "abandoned" => LearningSessionStatus::Abandoned,
        _ => LearningSessionStatus::InProgress,
    }
}

#[cfg(test)]
mod tests {
    use super::PgLearningRepository;
    use ame_learning_application::{LearningContractFixtures, exercise_goal_and_journey_contract};
    use sqlx::migrate::Migrator;
    use sqlx::postgres::PgPoolOptions;

    static MIGRATOR: Migrator = sqlx::migrate!("../../db/migrations");

    #[tokio::test]
    async fn postgres_repository_satisfies_contract_on_clean_database() {
        if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
            eprintln!("skipping clean PostgreSQL contract test; set AME_RUN_DB_TESTS=1");
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

        let defaults = LearningContractFixtures::default();
        let fixtures = LearningContractFixtures {
            source_actor_id: defaults.subject_user_id,
            ..defaults
        };
        let mut transaction = pool.begin().await.expect("begin identity transaction");
        sqlx::query("SET CONSTRAINTS ALL DEFERRED")
            .execute(&mut *transaction)
            .await
            .expect("defer circular identity constraint");
        sqlx::query(
            "INSERT INTO tb_users (id, email, email_canonical, display_name, role)
             VALUES ($1, $2, $2, $3, 'learner')",
        )
        .bind(fixtures.subject_user_id)
        .bind(format!(
            "contract-{}@example.test",
            fixtures.subject_user_id
        ))
        .bind("Contract Learner")
        .execute(&mut *transaction)
        .await
        .expect("create contract learner");
        sqlx::query(
            "INSERT INTO tb_identities (id, identity_type, owner_user_id, label)
             VALUES ($1, 'human', $1, 'contract learner')",
        )
        .bind(fixtures.source_actor_id)
        .execute(&mut *transaction)
        .await
        .expect("create human actor identity");
        transaction
            .commit()
            .await
            .expect("commit contract identity");

        exercise_goal_and_journey_contract(&PgLearningRepository::new(pool), fixtures).await;
    }
}
