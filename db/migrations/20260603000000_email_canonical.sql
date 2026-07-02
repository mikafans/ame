-- Add email_canonical column
ALTER TABLE tb_users ADD COLUMN email_canonical text;

-- Create function to compute canonical email
CREATE OR REPLACE FUNCTION tg_tb_users_email_canonical()
RETURNS trigger AS $$
BEGIN
    IF NEW.email IS NOT NULL THEN
        NEW.email_canonical := lower(split_part(split_part(NEW.email, '@', 1), '+', 1)) || '@' || lower(split_part(NEW.email, '@', 2));
    ELSE
        NEW.email_canonical := NULL;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create BEFORE INSERT OR UPDATE trigger to automatically keep email_canonical in sync
CREATE TRIGGER trg_tb_users_email_canonical
BEFORE INSERT OR UPDATE OF email ON tb_users
FOR EACH ROW
EXECUTE FUNCTION tg_tb_users_email_canonical();

-- Backfill existing rows (this will trigger trg_tb_users_email_canonical)
UPDATE tb_users SET email = email
WHERE email IS NOT NULL;

-- Create partial unique index to enforce uniqueness for non-agent users
CREATE UNIQUE INDEX idx_tb_users_email_canonical ON tb_users (email_canonical) WHERE email_canonical IS NOT NULL;
