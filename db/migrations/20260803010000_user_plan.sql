-- Restore the owner service tier used by the v0.1 rate-limit contract.
-- Registration defaults to free; administrators may promote an account to
-- premium from the Admin users panel.

ALTER TABLE tb_users
ADD COLUMN plan text NOT NULL DEFAULT 'free'
CHECK (plan IN ('free', 'premium'));
