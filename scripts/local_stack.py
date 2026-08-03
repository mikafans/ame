"""Run the containerized local AME stack.

Usage:
    uv run scripts/local_stack.py up
    uv run scripts/local_stack.py down
    uv run scripts/local_stack.py reset
    uv run scripts/local_stack.py logs
    uv run scripts/local_stack.py seed
    uv run scripts/local_stack.py admin
    uv run scripts/local_stack.py shell
    uv run scripts/local_stack.py uiux
    uv run scripts/local_stack.py api-contracts
"""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
import time
import json
import urllib.error
import urllib.request
import uuid
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


def promote_admin(email: str | None = None, name: str | None = None) -> None:
    email = email or os.environ.get("ADMIN_EMAIL", "admin@example.com")
    name = name or os.environ.get("ADMIN_NAME", "Carol Admin")
    password = os.environ.get("ADMIN_PASSWORD", "password123")
    password_hash = subprocess.check_output(
        [
            "uvx",
            "--quiet",
            "--from",
            "argon2-cffi",
            "python",
            "-c",
            f"from argon2 import PasswordHasher; print(PasswordHasher().hash({password!r}))",
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
  WHERE email_canonical = {email!r};

  IF existing_user_id IS NULL THEN
    new_user_id := uuid_generate_v7();
    INSERT INTO tb_identities (id, identity_type, label)
    VALUES (new_user_id, 'human', {name!r});
    INSERT INTO tb_users (id, email, email_canonical, display_name, role, password_hash)
    VALUES (new_user_id, {email!r}, {email!r}, {name!r}, 'admin', '{password_hash}');
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


def fixture_account() -> None:
    """Create the disposable simulation learner and its explicit fixture journey."""
    email = "fixture-simulation@example.test"
    password = "password123"
    registration = urllib.request.Request(
        "http://localhost:28800/public/v1/auth/register",
        data=json.dumps(
            {"email": email, "name": "Fixture Simulation", "password": password}
        ).encode(),
        method="POST",
        headers={"content-type": "application/json"},
    )
    try:
        with urllib.request.urlopen(registration, timeout=30) as response:
            if response.status != 201:
                raise SystemExit("fixture registration did not create an account")
    except urllib.error.HTTPError as error:
        if error.code != 422:
            raise SystemExit(f"fixture registration failed: {error.code}") from error

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
        """
        INSERT INTO tb_fixture_accounts (user_id, fixture_key)
        SELECT id, 'local-simulation'
        FROM tb_users
        WHERE email_canonical = 'fixture-simulation@example.test'
        ON CONFLICT (user_id) DO NOTHING;
        """,
    )
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
        """
        WITH fixture AS (
            SELECT id FROM tb_users WHERE email_canonical = 'fixture-simulation@example.test'
        ), goal AS (
            INSERT INTO tb_learning_goals (
                subject_user_id, source_actor_id, raw_intent, normalized_statement
            )
            SELECT id, id, 'fixture simulation', 'Fixture simulation'
            FROM fixture
            WHERE NOT EXISTS (
                SELECT 1
                FROM tb_learning_journeys j
                JOIN tb_users u ON u.id = j.subject_user_id
                WHERE u.email_canonical = 'fixture-simulation@example.test'
                  AND j.origin = 'fixture'
            )
            RETURNING id, subject_user_id
        )
        INSERT INTO tb_learning_journeys (
            goal_id, subject_user_id, source_actor_id, promise, origin
        )
        SELECT id, subject_user_id, subject_user_id, 'Fixture-only simulation', 'fixture'
        FROM goal;
        """,
    )
    print(f"fixture learner ready → {email} / {password}")


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


def run_learner_workspace() -> None:
    email = f"workspace-smoke-{uuid.uuid4()}@example.test"
    password = "workspace-smoke-2026"
    base_env = {
        **os.environ,
        "AME_API_URL": "http://localhost:28800",
        "HARU_REFERENCE_EMAIL": email,
        "HARU_REFERENCE_PASSWORD": password,
    }
    subprocess.run(
        [
            "uv",
            "run",
            "scripts/seed_reference_courses.py",
            "--complete-course",
            "flink-clickstream",
        ],
        cwd=ROOT,
        env=base_env,
        check=True,
    )
    subprocess.run(
        [
            "bunx",
            "playwright",
            "test",
            "e2e/learner-workspace.spec.ts",
            "--project=chromium",
        ],
        cwd=ROOT / "web",
        env={
            **base_env,
            "PORT": "28800",
            "NEXT_PUBLIC_API_URL": "http://localhost:28800",
            "E2E_API_URL": "http://localhost:28800",
            "E2E_BASE_URL": "http://localhost:28800",
            "E2E_EXTERNAL_SERVER": "1",
            "E2E_REFERENCE_EMAIL": email,
            "E2E_REFERENCE_PASSWORD": password,
        },
        check=True,
    )


def run_api_contracts() -> None:
    env = {**os.environ, "AME_API_URL": "http://localhost:28800"}
    subprocess.run(
        ["uv", "run", "pytest", "api_tests", "-v"],
        cwd=ROOT,
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
    # haru is the demo learner; also grant admin so the same account can see
    # both the learner desk and the admin console without switching accounts.
    promote_admin(email="haru@example.com", name="Haru")
    fixture_account()


def bring_up() -> None:
    # The web source and its node_modules live in separate mounts. Recreate
    # the web container on each start so a changed package manifest cannot
    # leave a stale dependency volume behind.
    compose("up", "-d", "--build", "--force-recreate", "api", "web", "caddy")
    wait_for_api()
    wait_for_web()
    seed_stack()
    print("local stack ready → http://localhost:28800")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "command",
        choices=(
            "up",
            "down",
            "reset",
            "logs",
            "seed",
            "admin",
            "shell",
            "uiux",
            "api-contracts",
            "learner-workspace",
        ),
    )
    args = parser.parse_args()

    if args.command == "up":
        bring_up()
    elif args.command == "down":
        compose("down")
    elif args.command == "reset":
        compose("down", "-v")
        bring_up()
    elif args.command == "logs":
        compose("logs", "-f")
    elif args.command == "seed":
        wait_for_api()
        seed_stack(force=True)
    elif args.command == "admin":
        promote_admin()
    elif args.command == "shell":
        compose("exec", "postgres", "psql", "-U", "postgres", "-d", "ame")
    elif args.command == "api-contracts":
        wait_for_api()
        run_api_contracts()
    elif args.command == "learner-workspace":
        wait_for_api()
        run_learner_workspace()
    else:
        wait_for_api()
        run_uiux()
    return 0


if __name__ == "__main__":
    sys.exit(main())
