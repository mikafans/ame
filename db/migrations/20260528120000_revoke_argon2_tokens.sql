-- Token hashing migrated from Argon2 (variable-length PHC string starting
-- with `$argon2`) to lowercase-hex SHA-256 (64 chars). Existing rows cannot
-- be verified by the new code path, so revoke them. Users will be issued a
-- fresh token on their next login.

UPDATE tb_api_tokens
SET revoked_at = now()
WHERE revoked_at IS NULL
  AND token_hash LIKE '$argon2%';
