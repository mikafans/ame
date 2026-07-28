//! Immutable source snapshot repository port.

use crate::domain::source::{ImportSource, SourceError, SourceImportStatus, SourceKind};
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub id: Uuid,
    pub subject_user_id: Uuid,
    pub kind: SourceKind,
    pub locator: String,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceImportRun {
    pub id: Uuid,
    pub source_id: Uuid,
    pub retry_key: String,
    pub status: SourceImportStatus,
    pub error_code: Option<String>,
    pub created_at: OffsetDateTime,
    pub completed_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSnapshot {
    pub id: Uuid,
    pub source_id: Uuid,
    pub import_run_id: Uuid,
    pub subject_user_id: Uuid,
    pub media_type: String,
    pub content_sha256: String,
    pub byte_length: u32,
    pub content: Vec<u8>,
    pub created_at: OffsetDateTime,
}

#[async_trait]
pub trait SourceRepository: Send + Sync {
    async fn import(
        &self,
        input: ImportSource,
    ) -> Result<(Source, SourceImportRun, SourceSnapshot), SourceError>;

    async fn record_failure(
        &self,
        subject_user_id: Uuid,
        source_kind: SourceKind,
        locator: String,
        retry_key: String,
        error_code: String,
    ) -> Result<SourceImportRun, SourceError>;

    async fn list_imports(
        &self,
        subject_user_id: Uuid,
    ) -> Result<Vec<SourceImportRun>, SourceError>;

    async fn get_snapshot(
        &self,
        subject_user_id: Uuid,
        snapshot_id: Uuid,
    ) -> Result<SourceSnapshot, SourceError>;

    async fn list_snapshots(
        &self,
        subject_user_id: Uuid,
    ) -> Result<Vec<SourceSnapshot>, SourceError>;
}

pub fn content_sha256(content: &[u8]) -> String {
    hex::encode(Sha256::digest(content))
}
