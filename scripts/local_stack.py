"""Run the containerized local AME stack.

Usage:
    uv run scripts/local_stack.py up
    uv run scripts/local_stack.py down
    uv run scripts/local_stack.py logs
    uv run scripts/local_stack.py seed
    uv run scripts/local_stack.py uiux
"""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
import time
from pathlib import Path

from container_runtime import compose as run_compose, engine

ROOT = Path(__file__).resolve().parents[1]
COMPOSE_FILE = ROOT / "docker-compose.local.yml"
def compose(*args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    if check:
        return run_compose(COMPOSE_FILE, *args)
    try:
        return run_compose(COMPOSE_FILE, *args)
    except subprocess.CalledProcessError as error:
        return subprocess.CompletedProcess(error.cmd, error.returncode)


def wait_for_api(timeout: int = 120) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = subprocess.run(
            ["curl", "-fsS", "http://localhost:28800/healthz"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        if result.returncode == 0:
            return
        time.sleep(1)
    raise SystemExit("local stack did not become ready within 120 seconds")


def wait_for_web(timeout: int = 120) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = subprocess.run(
            ["curl", "-fsS", "http://localhost:28800/"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        if result.returncode == 0:
            return
        time.sleep(1)
    raise SystemExit("local web stack did not become ready within 120 seconds")


def promote_admin() -> None:
    password_hash = subprocess.check_output(
        [
            "uvx",
            "--quiet",
            "--from",
            "argon2-cffi",
            "python",
            "-c",
            "from argon2 import PasswordHasher; print(PasswordHasher().hash('password123'))",
        ],
        cwd=ROOT,
        text=True,
    ).strip()
    sql = f"""
DO $$
DECLARE
  existing_user_id uuid;
  new_user_id uuid;
BEGIN
  SELECT id INTO existing_user_id
  FROM tb_users
  WHERE email_canonical = 'admin@example.com';

  IF existing_user_id IS NULL THEN
    new_user_id := uuid_generate_v7();
    INSERT INTO tb_identities (id, identity_type, label)
    VALUES (new_user_id, 'human', 'Carol Admin');
    INSERT INTO tb_users (id, email, email_canonical, display_name, role, password_hash)
    VALUES (new_user_id, 'admin@example.com', 'admin@example.com', 'Carol Admin', 'admin', '{password_hash}');
  ELSE
    UPDATE tb_users SET role = 'admin' WHERE id = existing_user_id;
  END IF;
END
$$;
"""
    compose(
        "exec",
        "-T",
        "postgres",
        "psql",
        "-U",
        "postgres",
        "-d",
        "ame",
        "-v",
        "ON_ERROR_STOP=1",
        "-c",
        sql,
    )


def run_uiux() -> None:
    env = {
        **os.environ,
        "PORT": "28800",
        "NEXT_PUBLIC_API_URL": "http://localhost:28800",
        "E2E_API_URL": "http://localhost:28800",
        "E2E_BASE_URL": "http://localhost:28800",
        "E2E_EXTERNAL_SERVER": "1",
    }
    subprocess.run(
        [
            "mise",
            "exec",
            "--",
            "bunx",
            "playwright",
            "test",
            "e2e/uiux.spec.ts",
            "--project=chromium",
        ],
        cwd=ROOT / "web",
        env=env,
        check=True,
    )


def database_has_users() -> bool:
    result = subprocess.run(
        [
            *engine(),
            "-f",
            str(COMPOSE_FILE),
            "exec",
            "-T",
            "postgres",
            "psql",
            "-U",
            "postgres",
            "-d",
            "ame",
            "-At",
            "-c",
            "SELECT count(*) FROM tb_users",
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    return result.returncode == 0 and result.stdout.strip() not in {"", "0"}


def seed_stack(*, force: bool = False) -> None:
    if not force and database_has_users():
        print("local database already contains users; skipping automatic seed")
        return
    promote_admin()
    subprocess.run(
        ["uv", "run", "scripts/seed_current.py", "--api", "http://localhost:28800"],
        cwd=ROOT,
        check=True,
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("up", "down", "logs", "seed", "uiux"))
    args = parser.parse_args()

    if args.command == "up":
        # The web source and its node_modules live in separate mounts. Recreate
        # the web container on each start so a changed package manifest cannot
        # leave a stale dependency volume behind.
        compose("up", "-d", "--build", "--force-recreate", "web", "caddy")
        wait_for_api()
        wait_for_web()
        seed_stack()
        print("local stack ready → http://localhost:28800")
    elif args.command == "down":
        compose("down")
    elif args.command == "logs":
        compose("logs", "-f")
    elif args.command == "seed":
        wait_for_api()
        seed_stack(force=True)
    else:
        wait_for_api()
        run_uiux()
    return 0


if __name__ == "__main__":
    sys.exit(main())
