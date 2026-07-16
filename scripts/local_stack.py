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
import shutil
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
COMPOSE_FILE = ROOT / "docker-compose.local.yml"
COMPOSE = os.environ.get(
    "COMPOSE",
    "podman compose" if shutil.which("podman") else "docker compose",
)


def compose(*args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [*COMPOSE.split(), "-f", str(COMPOSE_FILE), *args],
        cwd=ROOT,
        check=check,
        text=True,
    )


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
    sql = (
        "INSERT INTO tb_users (email, display_name, role, password_hash) "
        "VALUES ('admin@example.com', 'Carol Admin', 'admin', "
        f"'{password_hash}') "
        "ON CONFLICT (email) DO UPDATE SET role = 'admin';"
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
        sql,
    )


def run_uiux() -> None:
    env = {
        **os.environ,
        "PORT": "28800",
        "NEXT_PUBLIC_API_URL": "http://localhost:28800",
        "E2E_API_URL": "http://localhost:28800",
        "E2E_BASE_URL": "http://localhost:28800",
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
def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("up", "down", "logs", "seed", "uiux"))
    args = parser.parse_args()

    if args.command == "up":
        compose("up", "-d", "--build")
        wait_for_api()
        print("local stack ready → http://localhost:28800")
    elif args.command == "down":
        compose("down")
    elif args.command == "logs":
        compose("logs", "-f")
    elif args.command == "seed":
        wait_for_api()
        promote_admin()
        subprocess.run(
            ["uv", "run", "scripts/seed.py", "--api", "http://localhost:28800"],
            cwd=ROOT,
            check=True,
        )
    else:
        wait_for_api()
        run_uiux()
    return 0


if __name__ == "__main__":
    sys.exit(main())
