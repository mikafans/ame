//! Learning repositories and their contract tests.

use crate::domain::learning::{
    ActivityKind, ActivityStatus, AuthorActivityContent, CreateActivity, CreateGoal, CreateJourney,
    CreateLearningSession, CreateObjective, FinishLearningSession, GoalStatus, JourneyStatus,
    LearningActivity, LearningGoal, LearningJourney, LearningObjective, LearningRepository,
    LearningRepositoryError, LearningSession, LearningSessionStatus, ObjectiveStatus,
    validate_activity, validate_activity_content, validate_goal, validate_journey,
    validate_objective,
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
    sessions: HashMap<Uuid, LearningSession>,
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

    async fn list_journeys(
        &self,
        subject_user_id: Uuid,
    ) -> Result<Vec<LearningJourney>, LearningRepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(LearningRepositoryError::storage)?;
        let mut journeys: Vec<_> = state
            .journeys
            .values()
            .filter(|journey| journey.subject_user_id == subject_user_id)
            .cloned()
            .collect();
        journeys.sort_by_key(|journey| std::cmp::Reverse(journey.created_at));
        Ok(journeys)
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

    async fn author_activity_content(
        &self,
        input: AuthorActivityContent,
    ) -> Result<LearningActivity, LearningRepositoryError> {
        validate_activity_content(&input)?;
        let mut state = self
            .state
            .lock()
            .map_err(LearningRepositoryError::storage)?;
        let activity = state.activities.get_mut(&input.activity_id).ok_or(
            LearningRepositoryError::NotFound {
                resource: "activity",
            },
        )?;
        if activity.subject_user_id != input.subject_user_id {
            return Err(LearningRepositoryError::SubjectMismatch);
        }
        if !matches!(
            activity.kind,
            ActivityKind::Explanation | ActivityKind::Example
        ) {
            return Err(LearningRepositoryError::InvalidActivityContent);
        }
        if activity.status == ActivityStatus::Completed {
            return Err(LearningRepositoryError::ActivityContentCompleted);
        }
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
        activity.updated_at = OffsetDateTime::now_utc();
        Ok(activity.clone())
    }

    async fn start_learning_session(
        &self,
        input: CreateLearningSession,
    ) -> Result<LearningSession, LearningRepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(LearningRepositoryError::storage)?;
        let activity =
            state
                .activities
                .get(&input.activity_id)
                .ok_or(LearningRepositoryError::NotFound {
                    resource: "activity",
                })?;
        if activity.journey_id != input.journey_id
            || activity.subject_user_id != input.subject_user_id
        {
            return Err(LearningRepositoryError::SubjectMismatch);
        }
        if activity.status != ActivityStatus::Ready {
            return Err(LearningRepositoryError::ActivityNotReady);
        }
        let goal_id = state
            .journeys
            .get(&input.journey_id)
            .ok_or(LearningRepositoryError::NotFound {
                resource: "journey",
            })?
            .goal_id;
        state
            .journeys
            .get_mut(&input.journey_id)
            .expect("journey exists")
            .status = JourneyStatus::Active;
        if let Some(goal) = state.goals.get_mut(&goal_id) {
            goal.status = GoalStatus::Active;
        }
        if let Some(existing) = state.sessions.values().find(|session| {
            session.activity_id == input.activity_id
                && session.subject_user_id == input.subject_user_id
                && session.status == LearningSessionStatus::InProgress
        }) {
            return Ok(existing.clone());
        }
        let session = LearningSession {
            id: Uuid::now_v7(),
            journey_id: input.journey_id,
            activity_id: input.activity_id,
            subject_user_id: input.subject_user_id,
            actor_identity_id: input.actor_identity_id,
            status: LearningSessionStatus::InProgress,
            question_plan: input.question_plan,
            result: None,
            started_at: OffsetDateTime::now_utc(),
            finished_at: None,
        };
        state.sessions.insert(session.id, session.clone());
        Ok(session)
    }

    async fn get_learning_session(
        &self,
        subject_user_id: Uuid,
        session_id: Uuid,
    ) -> Result<LearningSession, LearningRepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(LearningRepositoryError::storage)?;
        match state.sessions.get(&session_id) {
            Some(session) if session.subject_user_id == subject_user_id => Ok(session.clone()),
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
        let state = self
            .state
            .lock()
            .map_err(LearningRepositoryError::storage)?;
        if state
            .journeys
            .get(&journey_id)
            .is_some_and(|journey| journey.subject_user_id != subject_user_id)
        {
            return Err(LearningRepositoryError::SubjectMismatch);
        }
        if !state.journeys.contains_key(&journey_id) {
            return Err(LearningRepositoryError::NotFound {
                resource: "journey",
            });
        }
        Ok(state
            .sessions
            .values()
            .filter(|session| {
                session.journey_id == journey_id
                    && session.subject_user_id == subject_user_id
                    && session.status == LearningSessionStatus::Finished
            })
            .max_by_key(|session| session.finished_at)
            .cloned())
    }

    async fn finish_learning_session(
        &self,
        input: FinishLearningSession,
    ) -> Result<LearningSession, LearningRepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(LearningRepositoryError::storage)?;
        let (activity_id, finished_session) = {
            let session = state.sessions.get_mut(&input.session_id).ok_or(
                LearningRepositoryError::NotFound {
                    resource: "learning session",
                },
            )?;
            if session.subject_user_id != input.subject_user_id {
                return Err(LearningRepositoryError::SubjectMismatch);
            }
            if session.status != LearningSessionStatus::InProgress {
                return match session.result.as_ref() {
                    Some(result) if *result == input.result => Ok(session.clone()),
                    Some(_) => Err(LearningRepositoryError::LearningSessionResultConflict),
                    None => Err(LearningRepositoryError::LearningSessionFinished),
                };
            }
            let activity_id = session.activity_id;
            session.status = if input.completed {
                LearningSessionStatus::Finished
            } else {
                LearningSessionStatus::Abandoned
            };
            session.result = Some(input.result);
            session.finished_at = Some(OffsetDateTime::now_utc());
            (activity_id, session.clone())
        };
        if !input.completed {
            return Ok(finished_session);
        }
        let (journey_id, order_index) = state
            .activities
            .get(&activity_id)
            .map(|activity| (activity.journey_id, activity.order_index))
            .ok_or(LearningRepositoryError::NotFound {
                resource: "activity",
            })?;
        if let Some(activity) = state.activities.get_mut(&activity_id) {
            activity.status = ActivityStatus::Completed;
            activity.updated_at = OffsetDateTime::now_utc();
        }
        let next_order = state
            .activities
            .values()
            .filter(|activity| {
                activity.journey_id == journey_id
                    && activity.order_index > order_index
                    && activity.status == ActivityStatus::Proposed
            })
            .map(|activity| activity.order_index)
            .min();
        if let Some(next_order) = next_order
            && let Some(activity) = state.activities.values_mut().find(|activity| {
                activity.journey_id == journey_id && activity.order_index == next_order
            })
        {
            activity.status = ActivityStatus::Ready;
            activity.updated_at = OffsetDateTime::now_utc();
        }
        Ok(finished_session)
    }
}

pub async fn exercise_goal_and_journey_contract<R: LearningRepository>(
    repository: &R,
    fixtures: LearningContractFixtures,
) {
    let subject = fixtures.subject_user_id;
    let other_subject = fixtures.other_subject_user_id;
    let actor = fixtures.source_actor_id;
    let idempotency_key = Some("onboarding/learner-selected-topic".to_string());
    let input = CreateGoal {
        subject_user_id: subject,
        source_actor_id: actor,
        template_version_id: None,
        raw_intent: "I would like to learn a learner-selected topic".to_string(),
        normalized_statement: "Understand the learner-selected topic".to_string(),
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
    let next_activity = repository
        .create_activity(CreateActivity {
            journey_id: journey.id,
            subject_user_id: subject,
            source_actor_id: actor,
            kind: crate::domain::learning::ActivityKind::Practice,
            title: "Interval practice".to_string(),
            order_index: 1,
            payload_schema_version: 1,
            payload: serde_json::json!({"count": 5}),
            objective_ids: vec![objective.id],
            status: ActivityStatus::Proposed,
        })
        .await
        .expect("next activity creates");
    assert_eq!(
        repository
            .list_activities(subject, journey.id)
            .await
            .expect("activities list"),
        vec![activity.clone(), next_activity]
    );
    assert_eq!(
        repository.list_activities(other_subject, journey.id).await,
        Err(LearningRepositoryError::SubjectMismatch)
    );
    let session_input = CreateLearningSession {
        journey_id: journey.id,
        activity_id: activity.id,
        subject_user_id: subject,
        actor_identity_id: actor,
        question_plan: serde_json::json!([]),
    };
    let session = repository
        .start_learning_session(session_input.clone())
        .await
        .expect("learning session starts");
    assert_eq!(session.status, LearningSessionStatus::InProgress);
    assert_eq!(
        repository
            .get_journey(subject, journey.id)
            .await
            .expect("active journey")
            .status,
        JourneyStatus::Active
    );
    assert_eq!(
        repository
            .get_goal(subject, goal.id)
            .await
            .expect("active goal")
            .status,
        GoalStatus::Active
    );
    assert_eq!(
        repository
            .start_learning_session(session_input.clone())
            .await
            .expect("learning session retry resumes"),
        session
    );
    assert_eq!(
        repository
            .get_learning_session(other_subject, session.id)
            .await,
        Err(LearningRepositoryError::SubjectMismatch)
    );
    let abandoned = repository
        .finish_learning_session(FinishLearningSession {
            subject_user_id: subject,
            session_id: session.id,
            completed: false,
            result: serde_json::json!({"completed": false}),
        })
        .await
        .expect("abandoned learning session records");
    assert_eq!(abandoned.status, LearningSessionStatus::Abandoned);
    assert_eq!(
        repository
            .list_activities(subject, journey.id)
            .await
            .expect("activities after abandon")[0]
            .status,
        ActivityStatus::Ready
    );
    let resumed_session = repository
        .start_learning_session(session_input.clone())
        .await
        .expect("abandoned session can be resumed");
    assert_ne!(resumed_session.id, session.id);
    let finished = repository
        .finish_learning_session(FinishLearningSession {
            subject_user_id: subject,
            session_id: resumed_session.id,
            completed: true,
            result: serde_json::json!({"completed": true}),
        })
        .await
        .expect("learning session finishes");
    assert_eq!(finished.status, LearningSessionStatus::Finished);
    assert_eq!(
        repository
            .list_activities(subject, journey.id)
            .await
            .expect("activities after finish")[1]
            .status,
        ActivityStatus::Ready
    );
    let finished_retry = repository
        .finish_learning_session(FinishLearningSession {
            subject_user_id: subject,
            session_id: resumed_session.id,
            completed: true,
            result: serde_json::json!({"completed": true}),
        })
        .await
        .expect("identical finish retry is idempotent");
    assert_eq!(finished_retry, finished);
    assert_eq!(
        repository
            .finish_learning_session(FinishLearningSession {
                subject_user_id: subject,
                session_id: resumed_session.id,
                completed: false,
                result: serde_json::json!({"completed": false}),
            })
            .await,
        Err(LearningRepositoryError::LearningSessionResultConflict)
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
