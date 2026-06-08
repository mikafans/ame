-- Interactive (browser) login sessions. Postgres is the source of truth; Valkey
-- (ame:login:{id}) is a fail-open read cache. Decoupled from tb_api_tokens (which
-- holds only PATs and agent tokens) so login survives a Valkey outage and login
-- tokens never surface in the admin token audit.
CREATE TABLE tb_login_sessions (
    id uuid PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    token_hash text NOT NULL,
    scopes text [] NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL
);

CREATE INDEX idx_login_sessions_user_id ON tb_login_sessions (user_id);
CREATE INDEX idx_login_sessions_expires_at ON tb_login_sessions (expires_at);
