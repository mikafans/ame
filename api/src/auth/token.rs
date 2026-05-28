//! API token format: `<uuid>_<hex-secret>`.
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

pub struct ParsedToken {
    pub id: Uuid,
    pub secret: String,
}

pub fn parse_bearer_token(bearer: &str) -> Option<ParsedToken> {
    let prefix = "Bearer ";
    if !bearer.starts_with(prefix) {
        return None;
    }
    let token = &bearer[prefix.len()..];
    let (id, secret) = token.split_once('_')?;
    if secret.is_empty() {
        return None;
    }
    let id = Uuid::parse_str(id).ok()?;
    Some(ParsedToken {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bearer_token_allows_underscores_in_secret() {
        let token_id = uuid::Uuid::now_v7();
        let parsed = parse_bearer_token(&format!("Bearer {token_id}_my_secret_token")).unwrap();

        assert_eq!(parsed.id, token_id);
        assert_eq!(parsed.secret, "my_secret_token");
    }

    #[test]
    fn parse_bearer_token_rejects_missing_secret() {
        let token_id = uuid::Uuid::now_v7();

        assert!(parse_bearer_token(&format!("Bearer {token_id}_")).is_none());
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
