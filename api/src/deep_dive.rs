//! Evidence-linked deep-dive repository contract.

use crate::domain::deep_dive::{CreateDeepDive, DeepDive, DeepDiveError, validate_deep_dive};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct InMemoryDeepDiveRepository {
    deep_dives: Arc<Mutex<HashMap<Uuid, DeepDive>>>,
}

impl InMemoryDeepDiveRepository {
    pub fn create(&self, input: CreateDeepDive) -> Result<DeepDive, DeepDiveError> {
        validate_deep_dive(&input)?;
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
        let created = repository
            .create(input(subject))
            .expect("deep-dive creates");
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
        let mut invalid = input(subject);
        invalid.source_references.clear();
        assert_eq!(
            repository.create(invalid),
            Err(DeepDiveError::MissingSource)
        );
        let created = repository
            .create(input(subject))
            .expect("deep-dive creates");
        assert_eq!(
            repository.get(Uuid::now_v7(), created.id),
            Err(DeepDiveError::SubjectMismatch)
        );
    }
}
