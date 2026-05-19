use argon2::{Argon2, password_hash::PasswordHash, password_hash::PasswordVerifier};
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
    let parts: Vec<&str> = token.split('_').collect();
    if parts.len() != 2 {
        return None;
    }
    let id = Uuid::parse_str(parts[0]).ok()?;
    Some(ParsedToken {
        id,
        secret: parts[1].to_string(),
    })
}

pub fn verify_token_secret(hash: &str, secret: &str) -> bool {
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(secret.as_bytes(), &parsed_hash)
        .is_ok()
}
