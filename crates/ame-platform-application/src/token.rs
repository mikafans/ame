//! Login token format: `lgn_<uuid>_<hex-secret>`.
//!
//! Secrets are server-generated 192-bit random values (48 hex chars). They are
//! stored as `sha256(secret)` and compared in constant time. We deliberately
//! do NOT use Argon2 here:
//!
//! - Argon2 is designed to slow down brute-force attacks against low-entropy
//!   human passwords. Our secrets are 192-bit cryptographic randoms; offline
//!   brute force is not the threat.
//! - Verifying every request through Argon2 (which we used to do) was a
//!   ~50–200 ms per-request CPU tax and a trivial DoS surface.
//!
//! Password hashing (login flow) is unchanged and still uses Argon2.

use rand::RngCore;
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Login,
}

impl TokenKind {
    pub fn prefix(&self) -> &'static str {
        match self {
            Self::Login => "lgn",
        }
    }

    pub fn parse(prefix: &str) -> Option<Self> {
        match prefix {
            "lgn" => Some(Self::Login),
            _ => None,
        }
    }
}

pub struct ParsedToken {
    pub kind: TokenKind,
    pub id: Uuid,
    pub secret: String,
}

pub fn parse_bearer_token(bearer: &str) -> Option<ParsedToken> {
    let prefix = "Bearer ";
    if !bearer.starts_with(prefix) {
        return None;
    }
    let token = &bearer[prefix.len()..];
    parse_token_value(token)
}

/// Parse a raw token value without the "Bearer " prefix.
/// Used for both Authorization header (after stripping "Bearer ") and Cookie header.
/// Expects `lgn_<uuid>_<secret>`.
pub fn parse_token_value(token: &str) -> Option<ParsedToken> {
    let mut parts = token.splitn(3, '_');
    let kind_str = parts.next()?;
    let id_str = parts.next()?;
    let secret = parts.next()?;

    if secret.is_empty() {
        return None;
    }

    let kind = TokenKind::parse(kind_str)?;
    let id = Uuid::parse_str(id_str).ok()?;

    Some(ParsedToken {
        kind,
        id,
        secret: secret.to_string(),
    })
}

/// Generate a fresh token secret. 24 bytes (192 bits) of entropy, hex-encoded.
pub fn generate_secret() -> String {
    let mut bytes = [0u8; 24];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Format a token as `<kind>_<uuid>_<secret>`.
pub fn format_token(kind: TokenKind, id: Uuid, secret: &str) -> String {
    format!("{}_{}_{}", kind.prefix(), id, secret)
}

/// Hash a token secret for storage. Returns the lowercase hex of `sha256(secret)`.
pub fn hash_secret(secret: &str) -> String {
    let digest = Sha256::digest(secret.as_bytes());
    hex::encode(digest)
}

/// Constant-time verify of a presented secret against the stored hash.
///
/// Returns `false` if the stored hash is not a valid hex sha256 (e.g. an old
/// argon2 string from before the migration), forcing a re-login.
pub fn verify_token_secret(stored_hash: &str, presented_secret: &str) -> bool {
    let stored = match hex::decode(stored_hash) {
        Ok(b) if b.len() == 32 => b,
        _ => return false,
    };
    let presented = Sha256::digest(presented_secret.as_bytes());
    stored.as_slice().ct_eq(presented.as_slice()).into()
}

/// Compute canonical email representation.
/// Normalizes case, trims whitespace, and strips '+' suffixes from the local part.
pub fn canonical_email(email: &str) -> String {
    let email_trimmed = email.trim();
    let parts: Vec<&str> = email_trimmed.split('@').collect();
    if parts.len() != 2 {
        return email_trimmed.to_lowercase();
    }
    let local = parts[0];
    let domain = parts[1];
    let local_without_plus = local.split('+').next().unwrap_or(local);
    format!(
        "{}@{}",
        local_without_plus.to_lowercase(),
        domain.to_lowercase()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_email() {
        assert_eq!(
            canonical_email("Ada.Lovelace+spam@Example.Com"),
            "ada.lovelace@example.com"
        );
        assert_eq!(
            canonical_email("  BOB+alias@domain.COM  "),
            "bob@domain.com"
        );
        assert_eq!(canonical_email("plain@domain.com"), "plain@domain.com");
        assert_eq!(canonical_email("invalid-email"), "invalid-email");
    }

    #[test]
    fn parse_bearer_token_allows_underscores_in_secret() {
        let token_id = uuid::Uuid::now_v7();
        let parsed = parse_bearer_token(&format!("Bearer lgn_{token_id}_my_secret_token")).unwrap();

        assert_eq!(parsed.kind, TokenKind::Login);
        assert_eq!(parsed.id, token_id);
        assert_eq!(parsed.secret, "my_secret_token");
    }

    #[test]
    fn parse_bearer_token_rejects_missing_prefix() {
        let token_id = uuid::Uuid::now_v7();
        assert!(parse_bearer_token(&format!("Bearer {token_id}_secret")).is_none());
    }

    #[test]
    fn parse_bearer_token_rejects_invalid_prefix() {
        let token_id = uuid::Uuid::now_v7();
        assert!(parse_bearer_token(&format!("Bearer bad_{token_id}_secret")).is_none());
    }

    #[test]
    fn parse_bearer_token_rejects_missing_secret() {
        let token_id = uuid::Uuid::now_v7();

        assert!(parse_bearer_token(&format!("Bearer lgn_{token_id}_")).is_none());
    }

    #[test]
    fn hash_and_verify_roundtrip() {
        let secret = generate_secret();
        let hash = hash_secret(&secret);
        assert!(verify_token_secret(&hash, &secret));
        assert!(!verify_token_secret(&hash, "wrong"));
    }

    #[test]
    fn verify_rejects_argon2_hashes() {
        // A real Argon2 PHC string starts with $argon2id$... and is not hex.
        // The new verifier must refuse it (forces a re-login).
        let argon2_hash = "$argon2id$v=19$m=19456,t=2,p=1$saltsaltsalt$hashhash";
        assert!(!verify_token_secret(argon2_hash, "anything"));
    }
}
