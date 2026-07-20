from __future__ import annotations

import os
import shutil
import subprocess
import time
from pathlib import Path

from .common import API_PORT, DB_URL, ROOT, compose, run


def up() -> int:
    return compose("up", "-d")


def down() -> int:
    return compose("down")


def migrate() -> int:
    return run("sqlx", "migrate", "run", "--source", "db/migrations", env={**os.environ, "DATABASE_URL": DB_URL})


def reset() -> int:
    if down() != 0:
        return 1
    data = ROOT / "db/data"
    try:
        shutil.rmtree(data)
    except FileNotFoundError:
        pass
    except PermissionError:
        if run("podman", "unshare", "rm", "-rf", str(data)) != 0:
            raise SystemExit("could not remove db/data")
    if up() != 0:
        return 1
    deadline = time.monotonic() + 120
    while time.monotonic() < deadline:
        if run("sqlx", "migrate", "info", "--source", "db/migrations", env={**os.environ, "DATABASE_URL": DB_URL}) == 0:
            compose("exec", "-T", "valkey", "valkey-cli", "flushall")
            return migrate()
        time.sleep(1)
    raise SystemExit("Postgres did not become ready")


def admin() -> int:
    email = os.environ.get("ADMIN_EMAIL", "admin@example.com")
    name = os.environ.get("ADMIN_NAME", "Carol Admin")
    password = os.environ.get("ADMIN_PASSWORD", "password123")
    hashed = subprocess.check_output(["uvx", "--quiet", "--from", "argon2-cffi", "python", "-c", f"from argon2 import PasswordHasher; print(PasswordHasher().hash({password!r}))"], text=True).strip()
    sql = f"""
BEGIN;
SET CONSTRAINTS ALL DEFERRED;
DO $$
DECLARE
  existing_user_id uuid;
  new_user_id uuid;
BEGIN
  SELECT id INTO existing_user_id FROM tb_users WHERE email_canonical = {email!r};
  IF existing_user_id IS NULL THEN
    new_user_id := uuid_generate_v7();
    INSERT INTO tb_users (id, email, email_canonical, display_name, role, password_hash)
    VALUES (new_user_id, {email!r}, {email!r}, {name!r}, 'admin', {hashed!r});
    INSERT INTO tb_identities (id, identity_type, owner_user_id, label)
    VALUES (new_user_id, 'human', new_user_id, {name!r});
  ELSE
    UPDATE tb_users
    SET display_name = {name!r}, role = 'admin', password_hash = {hashed!r}
    WHERE id = existing_user_id;
  END IF;
END
$$;
COMMIT;
"""
    return compose("exec", "-T", "postgres", "psql", "-U", "postgres", "-d", "ame", "-v", "ON_ERROR_STOP=1", "-c", sql)


def seed() -> int:
    return run("uv", "run", "scripts/seed_current.py", "--api", f"http://localhost:{API_PORT}")
