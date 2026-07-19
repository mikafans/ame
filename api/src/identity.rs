//! Identity repositories and self-host registration contract tests.

use crate::auth::token::canonical_email;
use crate::domain::identity::{
    AccountStatus, BrowserSession, CreateBrowserSession, CreateLearner, IdentityRepository,
    IdentityRepositoryError, LearnerAccount,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct InMemoryIdentityRepository {
    state: Arc<Mutex<State>>,
}

#[derive(Debug, Clone, Copy)]
pub struct IdentityContractFixtures {
    pub duplicate_email: &'static str,
    pub display_name: &'static str,
    pub token_hash: &'static str,
}

impl Default for IdentityContractFixtures {
    fn default() -> Self {
        Self {
            duplicate_email: "Ada.Lovelace+first@Example.COM",
            display_name: "Ada",
            token_hash: "hashed-session-secret",
        }
    }
}

#[derive(Default)]
struct State {
    accounts: HashMap<Uuid, LearnerAccount>,
    email_to_user: HashMap<String, Uuid>,
    sessions: HashMap<Uuid, BrowserSession>,
}

#[async_trait]
impl IdentityRepository for InMemoryIdentityRepository {
    async fn find_learner_by_email(
        &self,
        email: &str,
    ) -> Result<Option<LearnerAccount>, IdentityRepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(IdentityRepositoryError::storage)?;
        let canonical = canonical_email(email);
        Ok(state
            .email_to_user
            .get(&canonical)
            .and_then(|user_id| state.accounts.get(user_id))
            .cloned())
    }

    async fn create_learner(
        &self,
        input: CreateLearner,
    ) -> Result<LearnerAccount, IdentityRepositoryError> {
        if input.email.trim().is_empty() {
            return Err(IdentityRepositoryError::EmptyField { field: "email" });
        }
        if input.display_name.trim().is_empty() {
            return Err(IdentityRepositoryError::EmptyField {
                field: "display_name",
            });
        }

        let canonical = canonical_email(&input.email);
        let mut state = self
            .state
            .lock()
            .map_err(IdentityRepositoryError::storage)?;
        if state.email_to_user.contains_key(&canonical) {
            return Err(IdentityRepositoryError::AccountAlreadyExists);
        }

        let account = LearnerAccount {
            id: Uuid::now_v7(),
            email: canonical.clone(),
            display_name: input.display_name,
            status: AccountStatus::Active,
            created_at: OffsetDateTime::now_utc(),
        };
        state.email_to_user.insert(canonical, account.id);
        state.accounts.insert(account.id, account.clone());
        Ok(account)
    }

    async fn create_browser_session(
        &self,
        input: CreateBrowserSession,
    ) -> Result<BrowserSession, IdentityRepositoryError> {
        if input.token_hash.trim().is_empty() {
            return Err(IdentityRepositoryError::EmptyField {
                field: "token_hash",
            });
        }
        let mut state = self
            .state
            .lock()
            .map_err(IdentityRepositoryError::storage)?;
        if !state.accounts.contains_key(&input.user_id) {
            return Err(IdentityRepositoryError::AccountNotFound);
        }

        let session = BrowserSession {
            id: Uuid::now_v7(),
            user_id: input.user_id,
            token_hash: input.token_hash,
            expires_at: input.expires_at,
            revoked_at: None,
        };
        state.sessions.insert(session.id, session.clone());
        Ok(session)
    }

    async fn get_browser_session(
        &self,
        session_id: Uuid,
    ) -> Result<BrowserSession, IdentityRepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(IdentityRepositoryError::storage)?;
        state
            .sessions
            .get(&session_id)
            .cloned()
            .ok_or(IdentityRepositoryError::SessionNotFound)
    }
}

pub async fn exercise_identity_repository_contract<R: IdentityRepository>(
    repository: &R,
    fixtures: IdentityContractFixtures,
) {
    let account = repository
        .create_learner(CreateLearner {
            email: fixtures.duplicate_email.to_string(),
            display_name: fixtures.display_name.to_string(),
        })
        .await
        .expect("learner creates");
    assert_eq!(account.email, "ada.lovelace@example.com");
    assert_eq!(account.status, AccountStatus::Active);

    let existing = repository
        .find_learner_by_email("ada.lovelace@example.com")
        .await
        .expect("lookup succeeds")
        .expect("account exists");
    assert_eq!(existing.id, account.id);

    assert_eq!(
        repository
            .create_learner(CreateLearner {
                email: "ada.lovelace@example.com".to_string(),
                display_name: "Overwritten".to_string(),
            })
            .await,
        Err(IdentityRepositoryError::AccountAlreadyExists)
    );

    let session = repository
        .create_browser_session(CreateBrowserSession {
            user_id: account.id,
            token_hash: fixtures.token_hash.to_string(),
            expires_at: OffsetDateTime::now_utc() + time::Duration::hours(3),
        })
        .await
        .expect("session creates");
    assert_eq!(
        repository
            .get_browser_session(session.id)
            .await
            .expect("session reads"),
        session
    );
}

#[cfg(test)]
mod contract_tests {
    use super::{
        IdentityContractFixtures, InMemoryIdentityRepository, exercise_identity_repository_contract,
    };
    use crate::domain::identity::{
        CreateBrowserSession, CreateLearner, IdentityRepository, IdentityRepositoryError,
    };
    use time::OffsetDateTime;
    use uuid::Uuid;

    #[tokio::test]
    async fn open_registration_is_collision_safe_and_session_is_durable() {
        let repository = InMemoryIdentityRepository::default();
        exercise_identity_repository_contract(&repository, IdentityContractFixtures::default())
            .await;
    }

    #[tokio::test]
    async fn invalid_identity_inputs_are_rejected_without_partial_records() {
        let repository = InMemoryIdentityRepository::default();
        let account = repository
            .create_learner(CreateLearner {
                email: "learner@example.test".to_string(),
                display_name: "Learner".to_string(),
            })
            .await
            .expect("learner creates");

        assert_eq!(
            repository
                .create_browser_session(CreateBrowserSession {
                    user_id: Uuid::now_v7(),
                    token_hash: "token".to_string(),
                    expires_at: OffsetDateTime::now_utc(),
                })
                .await,
            Err(IdentityRepositoryError::AccountNotFound)
        );
        assert_eq!(
            repository
                .create_browser_session(CreateBrowserSession {
                    user_id: account.id,
                    token_hash: " ".to_string(),
                    expires_at: OffsetDateTime::now_utc(),
                })
                .await,
            Err(IdentityRepositoryError::EmptyField {
                field: "token_hash"
            })
        );
    }
}
