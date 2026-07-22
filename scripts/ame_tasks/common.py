from __future__ import annotations

import os
import subprocess
from pathlib import Path

from container_runtime import compose as run_compose

ROOT = Path(__file__).resolve().parents[2]
API_HOST = os.environ.get("API_HOST", "localhost")
API_PORT = os.environ.get("API_PORT", "28080")
WEB_PORT = os.environ.get("WEB_PORT", "23000")
PREVIEW_PORT = os.environ.get("PREVIEW_PORT", "28900")
DB_URL = os.environ.get(
    "AME_DATABASE_URL",
    "postgres://postgres:postgres@localhost:5432/ame",
)


def run(*args: str, cwd: Path = ROOT, env: dict[str, str] | None = None) -> int:
    return subprocess.run(args, cwd=cwd, env=env, check=False).returncode


def mise(*args: str, cwd: Path = ROOT, env: dict[str, str] | None = None) -> int:
    return run("mise", "exec", "--", *args, cwd=cwd, env=env)


def compose(*args: str) -> int:
    try:
        run_compose(ROOT / "db/docker-compose.yml", *args)
        return 0
    except subprocess.CalledProcessError as error:
        return error.returncode


def web_ready() -> bool:
    return (ROOT / "web/node_modules/.bin/next").exists()
