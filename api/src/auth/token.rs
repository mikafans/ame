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

pub fn verify_token_secret(hash: &str, secret: &str) -> bool {
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(secret.as_bytes(), &parsed_hash)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::parse_bearer_token;

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
}
