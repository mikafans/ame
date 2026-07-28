//! Citation anchors and publication grounding rules.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionMethod {
    ExactQuote,
    ManualSelection,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum GroundingStatus {
    Supported,
    Contradicted,
    Unverified,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LicenseStatus {
    Allowed,
    Restricted,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateCitation {
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
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CitationError {
    #[error("citation range must be non-empty and within the source snapshot")]
    InvalidRange,
    #[error("citation quote does not match the immutable snapshot range")]
    QuoteMismatch,
    #[error("citation field {field} must not be empty")]
    EmptyField { field: &'static str },
    #[error("citation is not supported by its source")]
    NotGrounded,
    #[error("citation source is restricted from publication")]
    RestrictedLicense,
    #[error("citation not found")]
    NotFound,
    #[error("citation storage failure: {0}")]
    Storage(String),
}

pub fn validate_anchor(input: &CreateCitation, snapshot: &[u8]) -> Result<(), CitationError> {
    if input.quote.trim().is_empty() {
        return Err(CitationError::EmptyField { field: "quote" });
    }
    if input.grounding_note.trim().is_empty() {
        return Err(CitationError::EmptyField {
            field: "grounding_note",
        });
    }
    let start = input.start_byte as usize;
    let end = input.end_byte as usize;
    if start >= end || end > snapshot.len() {
        return Err(CitationError::InvalidRange);
    }
    if snapshot.get(start..end) != Some(input.quote.as_bytes()) {
        return Err(CitationError::QuoteMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn citation(start_byte: u32, end_byte: u32, quote: &str) -> CreateCitation {
        CreateCitation {
            subject_user_id: Uuid::now_v7(),
            snapshot_id: Uuid::now_v7(),
            start_byte,
            end_byte,
            quote: quote.into(),
            extraction_method: ExtractionMethod::ExactQuote,
            grounding_status: GroundingStatus::Supported,
            grounding_note: "The quoted definition directly supports the claim.".into(),
            license_status: LicenseStatus::Allowed,
            license_name: Some("CC BY 4.0".into()),
            license_url: Some("https://creativecommons.org/licenses/by/4.0/".into()),
        }
    }

    #[test]
    fn certifies_exact_immutable_byte_range() {
        assert!(
            validate_anchor(
                &citation(0, 10, "Event time"),
                b"Event time uses event timestamps"
            )
            .is_ok()
        );
    }

    #[test]
    fn rejects_out_of_range_and_replaced_quotes() {
        assert_eq!(
            validate_anchor(&citation(0, 99, "Event time"), b"Event time"),
            Err(CitationError::InvalidRange)
        );
        assert_eq!(
            validate_anchor(&citation(0, 10, "Processing"), b"Event time"),
            Err(CitationError::QuoteMismatch)
        );
    }
}
