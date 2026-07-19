//! Learning repositories and their contract tests.

use crate::domain::learning::{
    ActivityStatus, CreateActivity, CreateGoal, CreateJourney, CreateObjective, GoalStatus,
    JourneyStatus, LearningActivity, LearningGoal, LearningJourney, LearningObjective,
    LearningRepository, LearningRepositoryError, ObjectiveStatus, validate_activity, validate_goal,
    validate_journey, validate_objective,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct InMemoryLearningRepository {
    state: Arc<Mutex<State>>,
}

#[derive(Debug, Clone, Copy)]
pub struct LearningContractFixtures {
    pub subject_user_id: Uuid,
    pub other_subject_user_id: Uuid,
    pub source_actor_id: Uuid,
}

impl Default for LearningContractFixtures {
    fn default() -> Self {
        Self {
            subject_user_id: Uuid::now_v7(),
            other_subject_user_id: Uuid::now_v7(),
            source_actor_id: Uuid::now_v7(),
        }
    }
}

#[derive(Default)]
struct State {
    goals: HashMap<Uuid, LearningGoal>,
    journeys: HashMap<Uuid, LearningJourney>,
    objectives: HashMap<Uuid, LearningObjective>,
    activities: HashMap<Uuid, LearningActivity>,
}

#[async_trait]
impl LearningRepository for InMemoryLearningRepository {
    async fn create_goal(
        &self,
        input: CreateGoal,
    ) -> Result<LearningGoal, LearningRepositoryError> {
        validate_goal(&input)?;
        let mut state = self
            .state
            .lock()
            .map_err(LearningRepositoryError::storage)?;

        if let Some(existing) = state.goals.values().find(|goal| {
            goal.subject_user_id == input.subject_user_id
                && goal.idempotency_key == input.idempotency_key
                && input.idempotency_key.is_some()
        }) {
            if existing.raw_intent == input.raw_intent
                && existing.normalized_statement == input.normalized_statement
                && existing.source_actor_id == input.source_actor_id
                && existing.template_version_id == input.template_version_id
            {
                return Ok(existing.clone());
            }
            return Err(LearningRepositoryError::IdempotencyConflict);
        }

        let goal = LearningGoal {
            id: Uuid::now_v7(),
            subject_user_id: input.subject_user_id,
            source_actor_id: input.source_actor_id,
            template_version_id: input.template_version_id,
            raw_intent: input.raw_intent,
            normalized_statement: input.normalized_statement,
            status: GoalStatus::Proposed,
            idempotency_key: input.idempotency_key,
            created_at: OffsetDateTime::now_utc(),
        };
        state.goals.insert(goal.id, goal.clone());
        Ok(goal)
    }

    async fn get_goal(
        &self,
        subject_user_id: Uuid,
        goal_id: Uuid,
    ) -> Result<LearningGoal, LearningRepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(LearningRepositoryError::storage)?;
        match state.goals.get(&goal_id) {
            Some(goal) if goal.subject_user_id == subject_user_id => Ok(goal.clone()),
            Some(_) => Err(LearningRepositoryError::SubjectMismatch),
            None => Err(LearningRepositoryError::NotFound { resource: "goal" }),
        }
    }

    async fn ensure_journey(
        &self,
        input: CreateJourney,
    ) -> Result<LearningJourney, LearningRepositoryError> {
        validate_journey(&input)?;
        let mut state = self
            .state
            .lock()
            .map_err(LearningRepositoryError::storage)?;
        let goal = match state.goals.get(&input.goal_id) {
            Some(goal) if goal.subject_user_id == input.subject_user_id => goal,
            Some(_) => return Err(LearningRepositoryError::SubjectMismatch),
            None => return Err(LearningRepositoryError::NotFound { resource: "goal" }),
        };

        if let Some(existing) = state
            .journeys
            .values()
            .find(|journey| journey.goal_id == input.goal_id)
        {
            return Ok(existing.clone());
        }

        let journey = LearningJourney {
            id: Uuid::now_v7(),
            goal_id: goal.id,
            subject_user_id: input.subject_user_id,
            source_actor_id: input.source_actor_id,
            promise: input.promise,
            status: JourneyStatus::Onboarding,
            created_at: OffsetDateTime::now_utc(),
        };
        state.journeys.insert(journey.id, journey.clone());
        Ok(journey)
    }

    async fn get_journey(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
    ) -> Result<LearningJourney, LearningRepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(LearningRepositoryError::storage)?;
        match state.journeys.get(&journey_id) {
            Some(journey) if journey.subject_user_id == subject_user_id => Ok(journey.clone()),
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
        let mut state = self
            .state
            .lock()
            .map_err(LearningRepositoryError::storage)?;
        let journey = match state.journeys.get_mut(&journey_id) {
            Some(journey) if journey.subject_user_id == subject_user_id => journey,
            Some(_) => return Err(LearningRepositoryError::SubjectMismatch),
            None => {
                return Err(LearningRepositoryError::NotFound {
                    resource: "journey",
                });
            }
        };
        journey.status = status;
        Ok(journey.clone())
    }

    async fn create_objective(
        &self,
        input: CreateObjective,
    ) -> Result<LearningObjective, LearningRepositoryError> {
        validate_objective(&input)?;
        let mut state = self
            .state
            .lock()
            .map_err(LearningRepositoryError::storage)?;
        let journey =
            state
                .journeys
                .get(&input.journey_id)
                .ok_or(LearningRepositoryError::NotFound {
                    resource: "journey",
                })?;
        if journey.subject_user_id != input.subject_user_id {
            return Err(LearningRepositoryError::SubjectMismatch);
        }
        if state.objectives.values().any(|objective| {
            objective.journey_id == input.journey_id && objective.order_index == input.order_index
        }) {
            return Err(LearningRepositoryError::OrderConflict {
                resource: "objective",
            });
        }
        let now = OffsetDateTime::now_utc();
        let objective = LearningObjective {
            id: Uuid::now_v7(),
            journey_id: input.journey_id,
            subject_user_id: input.subject_user_id,
            verb: input.verb,
            statement: input.statement,
            success_criteria: input.success_criteria,
            order_index: input.order_index,
            status: ObjectiveStatus::Active,
            created_at: now,
        };
        state.objectives.insert(objective.id, objective.clone());
        Ok(objective)
    }

    async fn list_objectives(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
    ) -> Result<Vec<LearningObjective>, LearningRepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(LearningRepositoryError::storage)?;
        let journey = state
            .journeys
            .get(&journey_id)
            .ok_or(LearningRepositoryError::NotFound {
                resource: "journey",
            })?;
        if journey.subject_user_id != subject_user_id {
            return Err(LearningRepositoryError::SubjectMismatch);
        }
        let mut objectives: Vec<_> = state
            .objectives
            .values()
            .filter(|objective| objective.journey_id == journey_id)
            .cloned()
            .collect();
        objectives.sort_by_key(|objective| objective.order_index);
        Ok(objectives)
    }

    async fn create_activity(
        &self,
        input: CreateActivity,
    ) -> Result<LearningActivity, LearningRepositoryError> {
        validate_activity(&input)?;
        let mut state = self
            .state
            .lock()
            .map_err(LearningRepositoryError::storage)?;
        let journey =
            state
                .journeys
                .get(&input.journey_id)
                .ok_or(LearningRepositoryError::NotFound {
                    resource: "journey",
                })?;
        if journey.subject_user_id != input.subject_user_id {
            return Err(LearningRepositoryError::SubjectMismatch);
        }
        if state.activities.values().any(|activity| {
            activity.journey_id == input.journey_id && activity.order_index == input.order_index
        }) {
            return Err(LearningRepositoryError::OrderConflict {
                resource: "activity",
            });
        }
        for objective_id in &input.objective_ids {
            let objective =
                state
                    .objectives
                    .get(objective_id)
                    .ok_or(LearningRepositoryError::NotFound {
                        resource: "objective",
                    })?;
            if objective.journey_id != input.journey_id
                || objective.subject_user_id != input.subject_user_id
            {
                return Err(LearningRepositoryError::SubjectMismatch);
            }
        }
        let now = OffsetDateTime::now_utc();
        let activity = LearningActivity {
            id: Uuid::now_v7(),
            journey_id: input.journey_id,
            subject_user_id: input.subject_user_id,
            source_actor_id: input.source_actor_id,
            source_run_id: input.source_run_id,
            kind: input.kind,
            title: input.title,
            order_index: input.order_index,
            payload_schema_version: input.payload_schema_version,
            payload: input.payload,
            objective_ids: input.objective_ids,
            status: input.status,
            created_at: now,
            updated_at: now,
        };
        state.activities.insert(activity.id, activity.clone());
        Ok(activity)
    }

    async fn list_activities(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
    ) -> Result<Vec<LearningActivity>, LearningRepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(LearningRepositoryError::storage)?;
        let journey = state
            .journeys
            .get(&journey_id)
            .ok_or(LearningRepositoryError::NotFound {
                resource: "journey",
            })?;
        if journey.subject_user_id != subject_user_id {
            return Err(LearningRepositoryError::SubjectMismatch);
        }
        let mut activities: Vec<_> = state
            .activities
            .values()
            .filter(|activity| activity.journey_id == journey_id)
            .cloned()
            .collect();
        activities.sort_by_key(|activity| activity.order_index);
        Ok(activities)
    }
}

pub async fn exercise_goal_and_journey_contract<R: LearningRepository>(
    repository: &R,
    fixtures: LearningContractFixtures,
) {
    let subject = fixtures.subject_user_id;
    let other_subject = fixtures.other_subject_user_id;
    let actor = fixtures.source_actor_id;
    let idempotency_key = Some("onboarding/music-theory".to_string());
    let input = CreateGoal {
        subject_user_id: subject,
        source_actor_id: actor,
        template_version_id: None,
        raw_intent: "I would like to learn music theory".to_string(),
        normalized_statement: "Understand foundational music theory".to_string(),
        idempotency_key: idempotency_key.clone(),
    };

    let goal = repository
        .create_goal(input.clone())
        .await
        .expect("goal creates");
    assert_eq!(goal.status, GoalStatus::Proposed);
    assert_eq!(
        repository
            .get_goal(subject, goal.id)
            .await
            .expect("goal reads"),
        goal
    );
    assert_eq!(
        repository.create_goal(input).await.expect("retry resumes"),
        goal
    );

    let conflict = repository
        .create_goal(CreateGoal {
            raw_intent: "I would like to learn harmony".to_string(),
            ..CreateGoal {
                subject_user_id: subject,
                source_actor_id: actor,
                template_version_id: None,
                raw_intent: String::new(),
                normalized_statement: "Different goal".to_string(),
                idempotency_key,
            }
        })
        .await;
    assert_eq!(conflict, Err(LearningRepositoryError::IdempotencyConflict));

    assert_eq!(
        repository.get_goal(other_subject, goal.id).await,
        Err(LearningRepositoryError::SubjectMismatch)
    );

    let journey_input = CreateJourney {
        goal_id: goal.id,
        subject_user_id: subject,
        source_actor_id: actor,
        promise: "Identify notes, intervals, and basic chords".to_string(),
    };
    let journey = repository
        .ensure_journey(journey_input.clone())
        .await
        .expect("journey creates");
    assert_eq!(journey.status, JourneyStatus::Onboarding);
    assert_eq!(
        repository
            .ensure_journey(journey_input)
            .await
            .expect("journey retry resumes"),
        journey
    );
    assert_eq!(
        repository
            .get_journey(subject, journey.id)
            .await
            .expect("journey reads"),
        journey
    );

    let active = repository
        .set_journey_status(subject, journey.id, JourneyStatus::Active)
        .await
        .expect("journey updates");
    assert_eq!(active.status, JourneyStatus::Active);

    let objective = repository
        .create_objective(CreateObjective {
            journey_id: journey.id,
            subject_user_id: subject,
            verb: "identify".to_string(),
            statement: "Identify intervals by ear".to_string(),
            success_criteria: "Name eight of ten intervals correctly".to_string(),
            order_index: 0,
        })
        .await
        .expect("objective creates");
    assert_eq!(
        repository
            .list_objectives(subject, journey.id)
            .await
            .expect("objectives list"),
        vec![objective.clone()]
    );

    let activity = repository
        .create_activity(CreateActivity {
            journey_id: journey.id,
            subject_user_id: subject,
            source_actor_id: actor,
            source_run_id: None,
            kind: crate::domain::learning::ActivityKind::Diagnostic,
            title: "Interval diagnostic".to_string(),
            order_index: 0,
            payload_schema_version: 1,
            payload: serde_json::json!({"count": 10}),
            objective_ids: vec![objective.id],
            status: ActivityStatus::Ready,
        })
        .await
        .expect("activity creates");
    assert_eq!(
        repository
            .list_activities(subject, journey.id)
            .await
            .expect("activities list"),
        vec![activity]
    );
    assert_eq!(
        repository.list_activities(other_subject, journey.id).await,
        Err(LearningRepositoryError::SubjectMismatch)
    );
    assert_eq!(
        repository.get_journey(other_subject, journey.id).await,
        Err(LearningRepositoryError::SubjectMismatch)
    );
}

#[cfg(test)]
mod contract_tests {
    use super::{
        InMemoryLearningRepository, LearningContractFixtures, exercise_goal_and_journey_contract,
    };
    use crate::domain::learning::{CreateGoal, LearningRepository, LearningRepositoryError};
    use uuid::Uuid;

    #[tokio::test]
    async fn in_memory_repository_satisfies_goal_and_journey_contract() {
        exercise_goal_and_journey_contract(
            &InMemoryLearningRepository::default(),
            LearningContractFixtures::default(),
        )
        .await;
    }

    #[tokio::test]
    async fn invalid_goal_and_journey_inputs_are_rejected() {
        let repository = InMemoryLearningRepository::default();
        let subject = Uuid::now_v7();
        let actor = Uuid::now_v7();

        assert_eq!(
            repository
                .create_goal(CreateGoal {
                    subject_user_id: subject,
                    source_actor_id: actor,
                    template_version_id: None,
                    raw_intent: " ".to_string(),
                    normalized_statement: "Valid".to_string(),
                    idempotency_key: None,
                })
                .await,
            Err(LearningRepositoryError::EmptyField {
                field: "raw_intent"
            })
        );
    }
}
