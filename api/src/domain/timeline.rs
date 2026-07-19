//! Learner-owned chronological learning history.

use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineEventKind {
    ActivityStarted,
    ActivityCompleted,
    AssessmentSubmitted,
    EvidenceRecorded,
    DeepDiveCreated,
    StreakRecorded,
}

impl TimelineEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ActivityStarted => "activity_started",
            Self::ActivityCompleted => "activity_completed",
            Self::AssessmentSubmitted => "assessment_submitted",
            Self::EvidenceRecorded => "evidence_recorded",
            Self::DeepDiveCreated => "deep_dive_created",
            Self::StreakRecorded => "streak_recorded",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineEvent {
    pub id: Uuid,
    pub subject_user_id: Uuid,
    pub journey_id: Uuid,
    pub activity_id: Option<Uuid>,
    pub kind: TimelineEventKind,
    pub title: String,
    pub occurred_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineError {
    Storage(String),
}

#[async_trait]
pub trait TimelineRepository: Send + Sync {
    async fn list_events(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
    ) -> Result<Vec<TimelineEvent>, TimelineError>;
}

#[derive(Clone, Default)]
pub struct InMemoryTimelineRepository {
    events: Arc<Mutex<Vec<TimelineEvent>>>,
}

impl InMemoryTimelineRepository {
    pub fn push(&self, event: TimelineEvent) -> Result<(), TimelineError> {
        let mut events = self
            .events
            .lock()
            .map_err(|error| TimelineError::Storage(error.to_string()))?;
        events.push(event);
        events.sort_by_key(|event| event.occurred_at);
        Ok(())
    }

    pub fn list_events(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
    ) -> Result<Vec<TimelineEvent>, TimelineError> {
        let events = self
            .events
            .lock()
            .map_err(|error| TimelineError::Storage(error.to_string()))?;
        Ok(events
            .iter()
            .filter(|event| {
                event.subject_user_id == subject_user_id && event.journey_id == journey_id
            })
            .cloned()
            .collect())
    }
}

#[async_trait]
impl TimelineRepository for InMemoryTimelineRepository {
    async fn list_events(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
    ) -> Result<Vec<TimelineEvent>, TimelineError> {
        self.list_events(subject_user_id, journey_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Duration;

    fn event(
        subject_user_id: Uuid,
        journey_id: Uuid,
        kind: TimelineEventKind,
        occurred_at: OffsetDateTime,
    ) -> TimelineEvent {
        TimelineEvent {
            id: Uuid::now_v7(),
            subject_user_id,
            journey_id,
            activity_id: Some(Uuid::now_v7()),
            kind,
            title: kind.as_str().replace('_', " "),
            occurred_at,
        }
    }

    #[test]
    fn timeline_returns_owned_events_in_chronological_order() {
        let repository = InMemoryTimelineRepository::default();
        let subject = Uuid::now_v7();
        let journey = Uuid::now_v7();
        let now = OffsetDateTime::now_utc();
        repository
            .push(event(
                subject,
                journey,
                TimelineEventKind::ActivityCompleted,
                now + Duration::seconds(2),
            ))
            .expect("event stores");
        repository
            .push(event(
                subject,
                journey,
                TimelineEventKind::ActivityStarted,
                now,
            ))
            .expect("event stores");

        let events = repository
            .list_events(subject, journey)
            .expect("timeline reads");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].kind, TimelineEventKind::ActivityStarted);
        assert_eq!(events[1].kind, TimelineEventKind::ActivityCompleted);
    }

    #[test]
    fn timeline_excludes_other_subjects_and_journeys() {
        let repository = InMemoryTimelineRepository::default();
        let subject = Uuid::now_v7();
        let journey = Uuid::now_v7();
        repository
            .push(event(
                subject,
                journey,
                TimelineEventKind::EvidenceRecorded,
                OffsetDateTime::now_utc(),
            ))
            .expect("event stores");

        assert!(
            repository
                .list_events(Uuid::now_v7(), journey)
                .expect("cross-subject read")
                .is_empty()
        );
        assert!(
            repository
                .list_events(subject, Uuid::now_v7())
                .expect("cross-journey read")
                .is_empty()
        );
    }

    #[test]
    fn timeline_kind_names_are_stable_contract_values() {
        assert_eq!(
            TimelineEventKind::AssessmentSubmitted.as_str(),
            "assessment_submitted"
        );
        assert_eq!(
            TimelineEventKind::StreakRecorded.as_str(),
            "streak_recorded"
        );
    }
}
