from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
API_HOST = os.environ.get("API_HOST", "localhost")
API_PORT = os.environ.get("API_PORT", "28080")
WEB_PORT = os.environ.get("WEB_PORT", "23000")
COMPOSE = os.environ.get(
    "COMPOSE", "podman compose" if shutil.which("podman") else "docker compose"
).split()
DB_URL = "postgres://postgres:postgres@localhost:5432/ame"


def run(*args: str, cwd: Path = ROOT, env: dict[str, str] | None = None) -> int:
    return subprocess.run(args, cwd=cwd, env=env, check=False).returncode


def mise(*args: str, cwd: Path = ROOT, env: dict[str, str] | None = None) -> int:
    return run("mise", "exec", "--", *args, cwd=cwd, env=env)


def compose(*args: str) -> int:
    return run(*COMPOSE, "-f", "db/docker-compose.yml", *args)


def web_ready() -> bool:
    return (ROOT / "web/node_modules/.bin/next").exists()
