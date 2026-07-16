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
    sql = f"INSERT INTO tb_users (email, display_name, role, password_hash) VALUES ({email!r}, {name!r}, 'admin', {hashed!r}) ON CONFLICT (email) DO UPDATE SET role = 'admin';"
    return compose("exec", "-T", "postgres", "psql", "-U", "postgres", "-d", "ame", "-v", "ON_ERROR_STOP=1", "-c", sql)


def seed() -> int:
    return run("uv", "run", "scripts/seed.py", "--api", f"http://localhost:{API_PORT}")


def bulk() -> int:
    return run("uv", "run", "scripts/mint_bulk.py", "--api", f"http://localhost:{API_PORT}")


def heavy() -> int:
    return reset() or seed() or bulk()
