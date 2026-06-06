-- Shorten the default API token lifetime from 30 days to 7 days.
-- A long-lived static bearer token is a standing liability if it leaks; 7 days
-- bounds the blast radius. Existing tokens keep their original expiry — this
-- only affects rows inserted after the migration. All three mint paths
-- (auth.rs issue_token, me.rs user token, me.rs agent token) inherit this
-- default; auth.rs no longer overrides it explicitly.
ALTER TABLE tb_api_tokens ALTER COLUMN expires_at SET DEFAULT now() + interval '7 days';
