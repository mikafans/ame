//! Citation repository port.

use crate::domain::citation::{
    CitationError, CreateCitation, ExtractionMethod, GroundingStatus, LicenseStatus,
};
use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Citation {
    pub id: Uuid,
    pub subject_user_id: Uuid,
    pub snapshot_id: Uuid,
    pub start_byte: u32,
    pub end_byte: u32,
    pub quote: String,
    pub extraction_method: ExtractionMethod,
    pub grounding_status: GroundingStatus,
    pub grounding_note: String,
    pub license_status: LicenseStatus,
    pub license_name: Option<String>,
    pub license_url: Option<String>,
    pub created_at: OffsetDateTime,
}

#[async_trait]
pub trait CitationRepository: Send + Sync {
    async fn create(&self, input: CreateCitation) -> Result<Citation, CitationError>;
    async fn get(&self, subject_user_id: Uuid, id: Uuid) -> Result<Citation, CitationError>;
    async fn list(&self, subject_user_id: Uuid) -> Result<Vec<Citation>, CitationError>;
    async fn certify_publication(
        &self,
        subject_user_id: Uuid,
        citation_ids: &[Uuid],
    ) -> Result<(), CitationError>;
}
