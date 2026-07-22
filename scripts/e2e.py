"""Run the host-backed Playwright suite with temporary API orchestration."""

from __future__ import annotations

import os
import signal
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
API_HOST = os.environ.get("API_HOST", "localhost")
API_PORT = os.environ.get("API_PORT", "28080")
WEB_PORT = os.environ.get("WEB_PORT", "23000")
API_URL = f"http://{API_HOST}:{API_PORT}"


def ready() -> bool:
    return subprocess.run(
        ["curl", "-fsS", f"{API_URL}/healthz"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    ).returncode == 0


def main() -> int:
    api: subprocess.Popen[str] | None = None
    try:
        if not ready():
            env = {
                **os.environ,
                "AME_DATABASE_URL": os.environ.get(
                    "AME_DATABASE_URL",
                    "postgres://postgres:postgres@localhost:5432/ame",
                ),
                "AME_PORT": API_PORT,
                "AME_CORS_ORIGINS": f"http://{API_HOST}:{WEB_PORT}",
                "AME_CONFIG_PATH": "ame.dev.toml",
                "RUST_LOG": "ame_api=debug,tower_http=info,sqlx=warn",
            }
            api = subprocess.Popen(
                [
                    "mise",
                    "exec",
                    "--",
                    "cargo",
                    "run",
                    "--manifest-path",
                    "api/Cargo.toml",
                    "--bin",
                    "ame-api",
                ],
                cwd=ROOT,
                env=env,
                start_new_session=True,
            )
            deadline = time.monotonic() + 90
            while not ready() and time.monotonic() < deadline:
                time.sleep(1)
            if not ready():
                raise SystemExit("API did not become ready for e2e")

        subprocess.run(["make", "db-admin"], cwd=ROOT, check=True)
        subprocess.run(
            ["uv", "run", "scripts/seed_current.py", "--api", API_URL],
            cwd=ROOT,
            check=True,
        )
        env = {
            **os.environ,
            "PORT": WEB_PORT,
            "NEXT_PUBLIC_API_URL": API_URL,
            "E2E_API_URL": API_URL,
            "E2E_BASE_URL": f"http://{API_HOST}:{WEB_PORT}",
        }
        return subprocess.run(
            ["mise", "exec", "--", "bun", "run", "e2e"],
            cwd=ROOT / "web",
            env=env,
            check=False,
        ).returncode
    finally:
        if api is not None:
            os.killpg(api.pid, signal.SIGTERM)
            api.wait(timeout=10)


if __name__ == "__main__":
    raise SystemExit(main())
