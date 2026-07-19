//! Evidence-backed mastery, recommendations, and streak contracts.

use crate::domain::progress::{
    MasteryEvidence, MasteryEvidenceInput, MasterySnapshot, ProgressError, Recommendation,
    StreakEvent, StreakEventInput, attempt_id_from_event_key, qualifying_day_for_attempt,
    validate_evidence, validate_streak_event,
};
use async_trait::async_trait;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct InMemoryProgressRepository {
    state: Arc<Mutex<ProgressState>>,
}

#[derive(Default)]
struct ProgressState {
    evidence: Vec<MasteryEvidence>,
    streak_events: HashMap<String, StreakEvent>,
    journey_owners: HashMap<Uuid, Uuid>,
    completed_attempts: HashMap<(Uuid, Uuid, Uuid, Uuid), OffsetDateTime>,
}

#[async_trait]
pub trait ProgressRepository: Send + Sync {
    async fn record_evidence(
        &self,
        input: MasteryEvidenceInput,
    ) -> Result<MasteryEvidence, ProgressError>;

    async fn snapshot(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
        objective_id: Uuid,
    ) -> Result<MasterySnapshot, ProgressError>;

    async fn recommend_weakest(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
        objectives: &[(Uuid, Uuid)],
    ) -> Result<Recommendation, ProgressError>;

    async fn record_streak_event(
        &self,
        input: StreakEventInput,
    ) -> Result<StreakEvent, ProgressError>;

    async fn list_streak_events(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
    ) -> Result<Vec<StreakEvent>, ProgressError>;
}

impl InMemoryProgressRepository {
    pub fn register_journey(&self, subject_user_id: Uuid, journey_id: Uuid) {
        if let Ok(mut state) = self.state.lock() {
            state.journey_owners.insert(journey_id, subject_user_id);
        }
    }

    pub fn register_completed_attempt(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
        activity_id: Uuid,
        attempt_id: Uuid,
        graded_at: OffsetDateTime,
    ) {
        if let Ok(mut state) = self.state.lock() {
            state.journey_owners.insert(journey_id, subject_user_id);
            state.completed_attempts.insert(
                (subject_user_id, journey_id, activity_id, attempt_id),
                graded_at,
            );
        }
    }

    pub fn record_evidence(
        &self,
        input: MasteryEvidenceInput,
    ) -> Result<MasteryEvidence, ProgressError> {
        validate_evidence(&input)?;
        let mut state = self
            .state
            .lock()
            .map_err(|error| ProgressError::Storage(error.to_string()))?;
        if let Some(owner_id) = state.journey_owners.get(&input.journey_id)
            && *owner_id != input.subject_user_id
        {
            return Err(ProgressError::SubjectMismatch);
        }
        let evidence = MasteryEvidence {
            id: Uuid::now_v7(),
            input,
            created_at: OffsetDateTime::now_utc(),
        };
        state.evidence.push(evidence.clone());
        Ok(evidence)
    }

    pub fn snapshot(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
        objective_id: Uuid,
    ) -> Result<MasterySnapshot, ProgressError> {
        let state = self
            .state
            .lock()
            .map_err(|error| ProgressError::Storage(error.to_string()))?;
        let evidence: Vec<_> = state
            .evidence
            .iter()
            .filter(|evidence| {
                evidence.input.subject_user_id == subject_user_id
                    && evidence.input.journey_id == journey_id
                    && evidence.input.objective_id == objective_id
            })
            .collect();
        if evidence.is_empty() {
            return Err(ProgressError::NotFound);
        }
        let mastery = evidence
            .iter()
            .map(|evidence| evidence.input.value)
            .sum::<f32>()
            / evidence.len() as f32;
        let confidence = (evidence.len() as f32 / 3.0).min(1.0);
        Ok(MasterySnapshot {
            subject_user_id,
            journey_id,
            objective_id,
            mastery,
            confidence,
            evidence_count: evidence.len() as u32,
            calculated_at: OffsetDateTime::now_utc(),
        })
    }

    pub fn recommend_weakest(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
        objectives: &[(Uuid, Uuid)],
    ) -> Result<Recommendation, ProgressError> {
        let mut candidates = Vec::new();
        for (objective_id, activity_id) in objectives {
            if let Ok(snapshot) = self.snapshot(subject_user_id, journey_id, *objective_id) {
                candidates.push((
                    snapshot.mastery,
                    snapshot.evidence_count,
                    *objective_id,
                    *activity_id,
                ));
            } else {
                candidates.push((0.0, 0, *objective_id, *activity_id));
            }
        }
        let (_, _, objective_id, activity_id) = candidates
            .into_iter()
            .min_by(|left, right| {
                left.0
                    .partial_cmp(&right.0)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| left.1.cmp(&right.1))
                    .then_with(|| left.2.cmp(&right.2))
            })
            .ok_or(ProgressError::NotFound)?;
        let evidence_ids = self
            .state
            .lock()
            .map_err(|error| ProgressError::Storage(error.to_string()))?
            .evidence
            .iter()
            .filter(|evidence| {
                evidence.input.subject_user_id == subject_user_id
                    && evidence.input.journey_id == journey_id
                    && evidence.input.objective_id == objective_id
            })
            .map(|evidence| evidence.id)
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

    pub fn record_streak_event(
        &self,
        input: StreakEventInput,
    ) -> Result<StreakEvent, ProgressError> {
        validate_streak_event(&input)?;
        let mut state = self
            .state
            .lock()
            .map_err(|error| ProgressError::Storage(error.to_string()))?;
        if let Some(owner_id) = state.journey_owners.get(&input.journey_id)
            && *owner_id != input.subject_user_id
        {
            return Err(ProgressError::SubjectMismatch);
        }
        let attempt_id = attempt_id_from_event_key(&input.qualifying_event_key)?;
        let Some(graded_at) = state.completed_attempts.get(&(
            input.subject_user_id,
            input.journey_id,
            input.activity_id,
            attempt_id,
        )) else {
            return Err(ProgressError::SubjectMismatch);
        };
        if qualifying_day_for_attempt(*graded_at, &input.learner_timezone)? != input.qualifying_day
        {
            return Err(ProgressError::InvalidStreakDay);
        }
        if let Some(event) = state.streak_events.get(&input.qualifying_event_key) {
            if event.input == input {
                return Ok(event.clone());
            }
            return Err(ProgressError::DuplicateStreakEvent);
        }
        let event = StreakEvent {
            id: Uuid::now_v7(),
            input: input.clone(),
            created_at: OffsetDateTime::now_utc(),
        };
        state
            .streak_events
            .insert(input.qualifying_event_key, event.clone());
        Ok(event)
    }

    pub fn streak_events_for_subject(
        &self,
        subject_user_id: Uuid,
    ) -> Result<Vec<StreakEvent>, ProgressError> {
        Ok(self
            .state
            .lock()
            .map_err(|error| ProgressError::Storage(error.to_string()))?
            .streak_events
            .values()
            .filter(|event| event.input.subject_user_id == subject_user_id)
            .cloned()
            .collect())
    }

    pub fn streak_events_for_journey(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
    ) -> Result<Vec<StreakEvent>, ProgressError> {
        let mut events = self
            .state
            .lock()
            .map_err(|error| ProgressError::Storage(error.to_string()))?
            .streak_events
            .values()
            .filter(|event| {
                event.input.subject_user_id == subject_user_id
                    && event.input.journey_id == journey_id
            })
            .cloned()
            .collect::<Vec<_>>();
        events.sort_by_key(|event| (event.input.qualifying_day, event.created_at));
        Ok(events)
    }
}

#[async_trait]
impl ProgressRepository for InMemoryProgressRepository {
    async fn record_evidence(
        &self,
        input: MasteryEvidenceInput,
    ) -> Result<MasteryEvidence, ProgressError> {
        self.record_evidence(input)
    }

    async fn snapshot(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
        objective_id: Uuid,
    ) -> Result<MasterySnapshot, ProgressError> {
        self.snapshot(subject_user_id, journey_id, objective_id)
    }

    async fn recommend_weakest(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
        objectives: &[(Uuid, Uuid)],
    ) -> Result<Recommendation, ProgressError> {
        self.recommend_weakest(subject_user_id, journey_id, objectives)
    }

    async fn record_streak_event(
        &self,
        input: StreakEventInput,
    ) -> Result<StreakEvent, ProgressError> {
        self.record_streak_event(input)
    }

    async fn list_streak_events(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
    ) -> Result<Vec<StreakEvent>, ProgressError> {
        self.streak_events_for_journey(subject_user_id, journey_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn evidence(subject: Uuid, objective: Uuid, value: f32) -> MasteryEvidenceInput {
        MasteryEvidenceInput {
            subject_user_id: subject,
            journey_id: Uuid::now_v7(),
            objective_id: objective,
            activity_id: Uuid::now_v7(),
            attempt_id: None,
            value,
            derivation_version: 1,
        }
    }

    #[test]
    fn mastery_is_rebuildable_and_recommendation_uses_evidence() {
        let repository = InMemoryProgressRepository::default();
        let subject = Uuid::now_v7();
        let journey = Uuid::now_v7();
        let weak = Uuid::now_v7();
        let strong = Uuid::now_v7();
        let mut weak_input = evidence(subject, weak, 0.25);
        weak_input.journey_id = journey;
        repository
            .record_evidence(weak_input)
            .expect("evidence records");
        let mut strong_input = evidence(subject, strong, 0.9);
        strong_input.journey_id = journey;
        repository
            .record_evidence(strong_input)
            .expect("evidence records");
        let snapshot = repository
            .snapshot(subject, journey, weak)
            .expect("snapshot calculates");
        assert_eq!(snapshot.mastery, 0.25);
        assert_eq!(snapshot.confidence, 1.0 / 3.0);
        let recommendation = repository
            .recommend_weakest(
                subject,
                journey,
                &[(strong, Uuid::now_v7()), (weak, Uuid::now_v7())],
            )
            .expect("recommendation calculates");
        assert_eq!(recommendation.objective_id, weak);
        assert_eq!(recommendation.evidence_ids.len(), 1);
    }

    #[test]
    fn invalid_or_cross_subject_evidence_cannot_create_progress() {
        let repository = InMemoryProgressRepository::default();
        let journey = Uuid::now_v7();
        let owner = Uuid::now_v7();
        repository.register_journey(owner, journey);
        let missing_attempt = evidence(owner, Uuid::now_v7(), 0.5);
        assert_eq!(
            repository.record_evidence(missing_attempt),
            Err(ProgressError::EmptyField {
                field: "attempt_id"
            })
        );
        let mut invalid = evidence(Uuid::now_v7(), Uuid::now_v7(), 1.1);
        assert_eq!(
            repository.record_evidence(invalid.clone()),
            Err(ProgressError::InvalidValue)
        );
        invalid.subject_user_id = owner;
        invalid.value = 0.5;
        invalid.journey_id = journey;
        repository
            .record_evidence(invalid)
            .expect("valid evidence records");
        let mut cross_owner = evidence(Uuid::now_v7(), journey, 0.5);
        cross_owner.journey_id = journey;
        cross_owner.activity_id = Uuid::now_v7();
        assert_eq!(
            repository.record_evidence(cross_owner),
            Err(ProgressError::SubjectMismatch)
        );
        assert_eq!(
            repository.snapshot(Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7()),
            Err(ProgressError::NotFound)
        );
    }

    #[test]
    fn streak_recording_is_idempotent_and_timezone_explicit() {
        use time::macros::datetime;

        let repository = InMemoryProgressRepository::default();
        let owner = Uuid::now_v7();
        let journey = Uuid::now_v7();
        let activity = Uuid::now_v7();
        let attempt = Uuid::now_v7();
        let graded_at = datetime!(2026-07-19 23:30 UTC);
        let input = StreakEventInput {
            subject_user_id: owner,
            journey_id: journey,
            activity_id: activity,
            qualifying_event_key: format!("attempt:{attempt}"),
            learner_timezone: "Asia/Tokyo".into(),
            qualifying_day: qualifying_day_for_attempt(graded_at, "Asia/Tokyo")
                .expect("qualifying day"),
        };
        repository.register_journey(owner, journey);
        repository.register_completed_attempt(owner, journey, activity, attempt, graded_at);
        let first = repository
            .record_streak_event(input.clone())
            .expect("streak records");
        assert_eq!(
            repository
                .record_streak_event(input.clone())
                .expect("retry is idempotent"),
            first
        );
        let mut conflict = input.clone();
        conflict.learner_timezone = "UTC".into();
        conflict.qualifying_day =
            qualifying_day_for_attempt(graded_at, "UTC").expect("UTC qualifying day");
        assert_eq!(
            repository.record_streak_event(conflict),
            Err(ProgressError::DuplicateStreakEvent)
        );
        let mut cross_owner = input;
        cross_owner.subject_user_id = Uuid::now_v7();
        assert_eq!(
            repository.record_streak_event(cross_owner),
            Err(ProgressError::SubjectMismatch)
        );
    }
}
