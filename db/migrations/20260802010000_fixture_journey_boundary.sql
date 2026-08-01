CREATE TABLE tb_fixture_accounts (
    user_id uuid PRIMARY KEY REFERENCES tb_users (id) ON DELETE CASCADE,
    fixture_key text NOT NULL UNIQUE,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tb_fixture_accounts_fixture_key_check CHECK (fixture_key <> '')
);

CREATE OR REPLACE FUNCTION fn_enforce_journey_origin()
RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND NEW.origin IS DISTINCT FROM OLD.origin THEN
        RAISE EXCEPTION 'learning journey origin is immutable';
    END IF;
    IF NEW.origin = 'fixture' AND NOT EXISTS (
        SELECT 1 FROM tb_fixture_accounts WHERE user_id = NEW.subject_user_id
    ) THEN
        RAISE EXCEPTION 'fixture journey requires a registered fixture account';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER tr_learning_journeys_origin
BEFORE INSERT OR UPDATE OF origin, subject_user_id ON tb_learning_journeys
FOR EACH ROW EXECUTE FUNCTION fn_enforce_journey_origin();
