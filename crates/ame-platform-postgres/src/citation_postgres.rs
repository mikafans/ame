//! PostgreSQL citation certificate adapter.

use crate::{
    citation::{Citation, CitationRepository},
    domain::citation::{
        CitationError, CreateCitation, ExtractionMethod, GroundingStatus, LicenseStatus,
        validate_anchor,
    },
};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct PgCitationRepository {
    pool: PgPool,
}

impl PgCitationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CitationRepository for PgCitationRepository {
    async fn create(&self, input: CreateCitation) -> Result<Citation, CitationError> {
        let snapshot = sqlx::query(
            "SELECT content FROM tb_source_snapshots
             WHERE id = $1 AND subject_user_id = $2",
        )
        .bind(input.snapshot_id)
        .bind(input.subject_user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage)?
        .ok_or(CitationError::NotFound)?;
        validate_anchor(&input, &snapshot.get::<Vec<u8>, _>("content"))?;
        let row = sqlx::query(
            "INSERT INTO tb_citations
                (id, subject_user_id, snapshot_id, start_byte, end_byte, quote,
                 extraction_method, grounding_status, grounding_note,
                 license_status, license_name, license_url)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
             RETURNING *",
        )
        .bind(Uuid::now_v7())
        .bind(input.subject_user_id)
        .bind(input.snapshot_id)
        .bind(input.start_byte as i32)
        .bind(input.end_byte as i32)
        .bind(input.quote)
        .bind(extraction_name(input.extraction_method))
        .bind(grounding_name(input.grounding_status))
        .bind(input.grounding_note)
        .bind(license_name(input.license_status))
        .bind(input.license_name)
        .bind(input.license_url)
        .fetch_one(&self.pool)
        .await
        .map_err(storage)?;
        map_citation(&row)
    }

    async fn get(&self, subject_user_id: Uuid, id: Uuid) -> Result<Citation, CitationError> {
        sqlx::query("SELECT * FROM tb_citations WHERE id = $1 AND subject_user_id = $2")
            .bind(id)
            .bind(subject_user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .map(|row| map_citation(&row))
            .transpose()?
            .ok_or(CitationError::NotFound)
    }

    async fn list(&self, subject_user_id: Uuid) -> Result<Vec<Citation>, CitationError> {
        sqlx::query(
            "SELECT * FROM tb_citations
             WHERE subject_user_id = $1 ORDER BY created_at DESC",
        )
        .bind(subject_user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(storage)?
        .iter()
        .map(map_citation)
        .collect()
    }

    async fn certify_publication(
        &self,
        subject_user_id: Uuid,
        citation_ids: &[Uuid],
    ) -> Result<(), CitationError> {
        if citation_ids.is_empty() {
            return Err(CitationError::EmptyField {
                field: "source_references",
            });
        }
        for id in citation_ids {
            let citation = self.get(subject_user_id, *id).await?;
            match citation.grounding_status {
                GroundingStatus::Supported => {}
                GroundingStatus::Contradicted | GroundingStatus::Unverified => {
                    return Err(CitationError::NotGrounded);
                }
            }
            if citation.license_status == LicenseStatus::Restricted {
                return Err(CitationError::RestrictedLicense);
            }
        }
        Ok(())
    }
}

fn map_citation(row: &sqlx::postgres::PgRow) -> Result<Citation, CitationError> {
    Ok(Citation {
        id: row.get("id"),
        subject_user_id: row.get("subject_user_id"),
        snapshot_id: row.get("snapshot_id"),
        start_byte: row.get::<i32, _>("start_byte") as u32,
        end_byte: row.get::<i32, _>("end_byte") as u32,
        quote: row.get("quote"),
        extraction_method: match row.get::<String, _>("extraction_method").as_str() {
            "exact_quote" => ExtractionMethod::ExactQuote,
            "manual_selection" => ExtractionMethod::ManualSelection,
            _ => return Err(CitationError::Storage("invalid extraction method".into())),
        },
        grounding_status: match row.get::<String, _>("grounding_status").as_str() {
            "supported" => GroundingStatus::Supported,
            "contradicted" => GroundingStatus::Contradicted,
            "unverified" => GroundingStatus::Unverified,
            _ => return Err(CitationError::Storage("invalid grounding status".into())),
        },
        grounding_note: row.get("grounding_note"),
        license_status: match row.get::<String, _>("license_status").as_str() {
            "allowed" => LicenseStatus::Allowed,
            "restricted" => LicenseStatus::Restricted,
            "unknown" => LicenseStatus::Unknown,
            _ => return Err(CitationError::Storage("invalid license status".into())),
        },
        license_name: row.get("license_name"),
        license_url: row.get("license_url"),
        created_at: row.get("created_at"),
    })
}

fn extraction_name(value: ExtractionMethod) -> &'static str {
    match value {
        ExtractionMethod::ExactQuote => "exact_quote",
        ExtractionMethod::ManualSelection => "manual_selection",
    }
}

fn grounding_name(value: GroundingStatus) -> &'static str {
    match value {
        GroundingStatus::Supported => "supported",
        GroundingStatus::Contradicted => "contradicted",
        GroundingStatus::Unverified => "unverified",
    }
}

fn license_name(value: LicenseStatus) -> &'static str {
    match value {
        LicenseStatus::Allowed => "allowed",
        LicenseStatus::Restricted => "restricted",
        LicenseStatus::Unknown => "unknown",
    }
}

fn storage(error: impl std::fmt::Display) -> CitationError {
    CitationError::Storage(error.to_string())
}
