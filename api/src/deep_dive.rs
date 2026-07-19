//! Evidence-linked deep-dive repository contract.

use crate::domain::deep_dive::{CreateDeepDive, DeepDive, DeepDiveError, validate_deep_dive};
use async_trait::async_trait;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};
use time::OffsetDateTime;
use uuid::Uuid;

type EvidenceKey = (Uuid, Uuid, Uuid, Uuid, Uuid);

#[derive(Clone, Default)]
pub struct InMemoryDeepDiveRepository {
    deep_dives: Arc<Mutex<HashMap<Uuid, DeepDive>>>,
    verified_evidence: Arc<Mutex<HashSet<EvidenceKey>>>,
}

#[async_trait]
pub trait DeepDiveRepository: Send + Sync {
    async fn create(&self, input: CreateDeepDive) -> Result<DeepDive, DeepDiveError>;
    async fn get(&self, subject_user_id: Uuid, id: Uuid) -> Result<DeepDive, DeepDiveError>;
    async fn get_for_activity(
        &self,
        subject_user_id: Uuid,
        activity_id: Uuid,
    ) -> Result<Option<DeepDive>, DeepDiveError>;
}

impl InMemoryDeepDiveRepository {
    pub fn register_evidence(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
        activity_id: Uuid,
        objective_id: Uuid,
        evidence_id: Uuid,
    ) -> Result<(), DeepDiveError> {
        self.verified_evidence
            .lock()
            .map_err(|error| DeepDiveError::Storage(error.to_string()))?
            .insert((
                subject_user_id,
                journey_id,
                activity_id,
                objective_id,
                evidence_id,
            ));
        Ok(())
    }

    pub fn create(&self, input: CreateDeepDive) -> Result<DeepDive, DeepDiveError> {
        validate_deep_dive(&input)?;
        let evidence_is_owned = self
            .verified_evidence
            .lock()
            .map_err(|error| DeepDiveError::Storage(error.to_string()))?
            .contains(&(
                input.subject_user_id,
                input.journey_id,
                input.activity_id,
                input.objective_id,
                input.triggering_evidence_id,
            ));
        if !evidence_is_owned {
            return Err(DeepDiveError::SubjectMismatch);
        }
        let mut deep_dives = self
            .deep_dives
            .lock()
            .map_err(|error| DeepDiveError::Storage(error.to_string()))?;
        let deep_dive = DeepDive {
            id: Uuid::now_v7(),
            input,
            content_version: 1,
            created_at: OffsetDateTime::now_utc(),
        };
        deep_dives.insert(deep_dive.id, deep_dive.clone());
        Ok(deep_dive)
    }

    pub fn get(&self, subject_user_id: Uuid, id: Uuid) -> Result<DeepDive, DeepDiveError> {
        let deep_dives = self
            .deep_dives
            .lock()
            .map_err(|error| DeepDiveError::Storage(error.to_string()))?;
        let deep_dive = deep_dives.get(&id).ok_or(DeepDiveError::NotFound)?;
        if deep_dive.input.subject_user_id != subject_user_id {
            return Err(DeepDiveError::SubjectMismatch);
        }
        Ok(deep_dive.clone())
    }
}

#[async_trait]
impl DeepDiveRepository for InMemoryDeepDiveRepository {
    async fn create(&self, input: CreateDeepDive) -> Result<DeepDive, DeepDiveError> {
        self.create(input)
    }

    async fn get(&self, subject_user_id: Uuid, id: Uuid) -> Result<DeepDive, DeepDiveError> {
        self.get(subject_user_id, id)
    }

    async fn get_for_activity(
        &self,
        subject_user_id: Uuid,
        activity_id: Uuid,
    ) -> Result<Option<DeepDive>, DeepDiveError> {
        let deep_dives = self
            .deep_dives
            .lock()
            .map_err(|error| DeepDiveError::Storage(error.to_string()))?;
        Ok(deep_dives
            .values()
            .find(|deep_dive| {
                deep_dive.input.activity_id == activity_id
                    && deep_dive.input.subject_user_id == subject_user_id
            })
            .cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{deep_dive::CreateDeepDive, question::ContentReviewStatus};

    fn input(subject: Uuid) -> CreateDeepDive {
        CreateDeepDive {
            subject_user_id: subject,
            source_actor_id: subject,
            journey_id: Uuid::now_v7(),
            activity_id: Uuid::now_v7(),
            objective_id: Uuid::now_v7(),
            triggering_evidence_id: Uuid::now_v7(),
            title: "A focused explanation".into(),
            body: "Explain the weak concept in learner language.".into(),
            example: "A minimal worked example.".into(),
            caveats: vec!["This does not change mastery by itself.".into()],
            source_references: vec!["https://example.test/source".into()],
            application_task: "Apply the concept in one small task.".into(),
            review_status: ContentReviewStatus::Review,
        }
    }

    #[test]
    fn deep_dive_is_linked_to_evidence_and_readable_by_owner() {
        let subject = Uuid::now_v7();
        let repository = InMemoryDeepDiveRepository::default();
        let value = input(subject);
        repository
            .register_evidence(
                value.subject_user_id,
                value.journey_id,
                value.activity_id,
                value.objective_id,
                value.triggering_evidence_id,
            )
            .expect("evidence registers");
        let created = repository.create(value).expect("deep-dive creates");
        assert_eq!(created.content_version, 1);
        assert_eq!(
            repository
                .get(subject, created.id)
                .expect("deep-dive reads"),
            created
        );
    }

    #[test]
    fn unsupported_or_cross_subject_deep_dives_are_rejected() {
        let subject = Uuid::now_v7();
        let repository = InMemoryDeepDiveRepository::default();
        assert_eq!(
            repository.create(input(subject)),
            Err(DeepDiveError::SubjectMismatch)
        );
        let mut invalid = input(subject);
        invalid.source_references.clear();
        assert_eq!(
            repository.create(invalid),
            Err(DeepDiveError::MissingSource)
        );
        let value = input(subject);
        repository
            .register_evidence(
                value.subject_user_id,
                value.journey_id,
                value.activity_id,
                value.objective_id,
                value.triggering_evidence_id,
            )
            .expect("evidence registers");
        let created = repository.create(value).expect("deep-dive creates");
        assert_eq!(
            repository.get(Uuid::now_v7(), created.id),
            Err(DeepDiveError::SubjectMismatch)
        );
    }
}
