-- Drop the foreign key constraint referencing tb_api_tokens to allow login session IDs to be used in idempotency keys
ALTER TABLE tb_idempotency_keys DROP CONSTRAINT IF EXISTS tb_idempotency_keys_token_id_fkey;
