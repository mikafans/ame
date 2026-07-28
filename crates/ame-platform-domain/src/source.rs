//! Immutable learner-owned source and import contracts.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

pub const MAX_SOURCE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Url,
    Document,
    LocalFile,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SourceImportStatus {
    Requested,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportSource {
    pub subject_user_id: Uuid,
    pub source_kind: SourceKind,
    pub locator: String,
    pub media_type: String,
    pub content: Vec<u8>,
    pub retry_key: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SourceError {
    #[error("source field {field} must not be empty")]
    EmptyField { field: &'static str },
    #[error("source content exceeds the two MiB limit")]
    TooLarge,
    #[error("source media type is not supported")]
    UnsupportedMediaType,
    #[error("source locator is not allowed")]
    UnsafeLocator,
    #[error("source import retry key conflicts with another payload")]
    RetryConflict,
    #[error("source snapshot not found")]
    NotFound,
    #[error("source belongs to another learner")]
    SubjectMismatch,
    #[error("source storage failure: {0}")]
    Storage(String),
}

pub fn validate_import(input: &ImportSource) -> Result<(), SourceError> {
    for (field, value) in [
        ("locator", input.locator.as_str()),
        ("media_type", input.media_type.as_str()),
        ("retry_key", input.retry_key.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(SourceError::EmptyField { field });
        }
    }
    if input.content.is_empty() {
        return Err(SourceError::EmptyField { field: "content" });
    }
    if input.content.len() > MAX_SOURCE_BYTES {
        return Err(SourceError::TooLarge);
    }
    if !matches!(
        input.media_type.as_str(),
        "text/plain" | "text/markdown" | "text/html" | "application/json"
    ) {
        return Err(SourceError::UnsupportedMediaType);
    }
    match input.source_kind {
        SourceKind::Url if !safe_https_locator(&input.locator) => Err(SourceError::UnsafeLocator),
        SourceKind::Document | SourceKind::LocalFile
            if input.locator.starts_with('/') || input.locator.contains("..") =>
        {
            Err(SourceError::UnsafeLocator)
        }
        _ => Ok(()),
    }
}

pub fn safe_https_locator(locator: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(locator) else {
        return false;
    };
    if url.scheme() != "https" || url.username() != "" || url.password().is_some() {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    let host = host
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if matches!(host.as_str(), "localhost" | "localhost.localdomain")
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || host.ends_with(".internal")
    {
        return false;
    }
    if let Ok(address) = host.parse::<std::net::IpAddr>() {
        return public_ip(address);
    }
    true
}

pub fn public_ip(address: std::net::IpAddr) -> bool {
    match address {
        std::net::IpAddr::V4(ip) => {
            !(ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_broadcast()
                || ip.is_documentation()
                || ip.is_unspecified()
                || ip.octets()[0] == 0
                || ip.octets()[0] >= 224)
        }
        std::net::IpAddr::V6(ip) => {
            !(ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_unique_local()
                || ip.is_unicast_link_local()
                || ip.is_multicast())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(kind: SourceKind, locator: &str) -> ImportSource {
        ImportSource {
            subject_user_id: Uuid::now_v7(),
            source_kind: kind,
            locator: locator.into(),
            media_type: "text/plain".into(),
            content: b"event time uses timestamps from events".to_vec(),
            retry_key: "flink-event-time-v1".into(),
        }
    }

    #[test]
    fn accepts_bounded_source_text() {
        assert!(
            validate_import(&input(
                SourceKind::Url,
                "https://nightlies.apache.org/flink/flink-docs-stable/docs/concepts/time/"
            ))
            .is_ok()
        );
        assert!(validate_import(&input(SourceKind::LocalFile, "notes/flink.md")).is_ok());
    }

    #[test]
    fn rejects_private_urls_and_filesystem_traversal() {
        for locator in [
            "http://example.com",
            "https://localhost/admin",
            "https://127.0.0.1/private",
            "https://169.254.169.254/latest/meta-data",
            "https://[::1]/private",
        ] {
            assert_eq!(
                validate_import(&input(SourceKind::Url, locator)),
                Err(SourceError::UnsafeLocator)
            );
        }
        assert_eq!(
            validate_import(&input(SourceKind::LocalFile, "../secret")),
            Err(SourceError::UnsafeLocator)
        );
    }
}
