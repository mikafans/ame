//! Evidence-backed mastery, recommendations, and streak contracts.

use crate::domain::progress::{
    MasteryEvidence, MasteryEvidenceInput, MasterySnapshot, ProgressError, Recommendation,
    StreakEvent, StreakEventInput, validate_evidence, validate_streak_event,
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
    pub fn record_evidence(
        &self,
        input: MasteryEvidenceInput,
    ) -> Result<MasteryEvidence, ProgressError> {
        validate_evidence(&input)?;
        let mut state = self
            .state
            .lock()
            .map_err(|error| ProgressError::Storage(error.to_string()))?;
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
    use time::macros::date;

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
        let mut invalid = evidence(Uuid::now_v7(), Uuid::now_v7(), 1.1);
        assert_eq!(
            repository.record_evidence(invalid.clone()),
            Err(ProgressError::InvalidValue)
        );
        invalid.value = 0.5;
        repository
            .record_evidence(invalid)
            .expect("valid evidence records");
        assert_eq!(
            repository.snapshot(Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7()),
            Err(ProgressError::NotFound)
        );
    }

    #[test]
    fn streak_recording_is_idempotent_and_timezone_explicit() {
        let repository = InMemoryProgressRepository::default();
        let input = StreakEventInput {
            subject_user_id: Uuid::now_v7(),
            journey_id: Uuid::now_v7(),
            activity_id: Uuid::now_v7(),
            qualifying_event_key: "activity:1:finish".into(),
            learner_timezone: "Asia/Tokyo".into(),
            qualifying_day: date!(2026 - 07 - 19),
        };
        let first = repository
            .record_streak_event(input.clone())
            .expect("streak records");
        assert_eq!(
            repository
                .record_streak_event(input.clone())
                .expect("retry is idempotent"),
            first
        );
        let mut conflict = input;
        conflict.learner_timezone = "UTC".into();
        assert_eq!(
            repository.record_streak_event(conflict),
            Err(ProgressError::DuplicateStreakEvent)
        );
    }
}
