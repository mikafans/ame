"""Small database backup/restore tasks for the Makefile wrapper."""

from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
COMPOSE = os.environ.get(
    "COMPOSE", "podman compose" if shutil.which("podman") else "docker compose"
).split()
BACKUP = Path(os.environ.get("BACKUP_FILE", "backup.dump"))
RESTORE_DB = os.environ.get("RESTORE_DB", "ame_scratch")


def compose(*args: str) -> int:
    return subprocess.run(
        [*COMPOSE, "-f", "db/docker-compose.yml", *args], cwd=ROOT, check=False
    ).returncode


def main() -> int:
    command = os.environ.get("DB_TASK", "")
    if not command:
        import sys

        command = sys.argv[1]
    if command == "backup":
        with BACKUP.open("wb") as output:
            return subprocess.run(
                [*COMPOSE, "-f", "db/docker-compose.yml", "exec", "-T", "postgres", "pg_dump", "-U", "postgres", "-d", "ame", "-Fc"],
                cwd=ROOT,
                stdout=output,
                check=False,
            ).returncode
    if command == "restore":
        if not BACKUP.is_file():
            raise SystemExit(f"backup not found: {BACKUP}")
        compose("exec", "-T", "postgres", "createdb", "-U", "postgres", RESTORE_DB)
        return subprocess.run(
            [*COMPOSE, "-f", "db/docker-compose.yml", "exec", "-T", "postgres", "pg_restore", "-U", "postgres", "-d", RESTORE_DB, "--clean", "--if-exists", "--no-owner", "--no-privileges"],
            cwd=ROOT,
            stdin=BACKUP.open("rb"),
            check=False,
        ).returncode
    raise SystemExit(f"unknown database task: {command}")


if __name__ == "__main__":
    raise SystemExit(main())
