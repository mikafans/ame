//! PostgreSQL implementation of the learning repository contract.

use crate::domain::learning::{
    CreateGoal, CreateJourney, GoalStatus, JourneyStatus, LearningGoal, LearningJourney,
    LearningRepository, LearningRepositoryError, validate_goal, validate_journey,
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

#[cfg(test)]
mod tests {
    use super::PgLearningRepository;
    use crate::learning::{LearningContractFixtures, exercise_goal_and_journey_contract};
    use sqlx::migrate::Migrator;
    use sqlx::postgres::PgPoolOptions;

    static MIGRATOR: Migrator = sqlx::migrate!("../db/migrations");

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
