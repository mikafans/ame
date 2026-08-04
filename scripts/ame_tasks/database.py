from __future__ import annotations

import os
import shutil
import time

from .common import DB_URL, ROOT, compose, run


def up() -> int:
    return compose("up", "-d")


def down() -> int:
    return compose("down")


def migrate() -> int:
    return run(
        "sqlx",
        "migrate",
        "run",
        "--source",
        "db/migrations",
        env={**os.environ, "DATABASE_URL": DB_URL},
    )


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
        if (
            run(
                "sqlx",
                "migrate",
                "info",
                "--source",
                "db/migrations",
                env={**os.environ, "DATABASE_URL": DB_URL},
            )
            == 0
        ):
            compose("exec", "-T", "valkey", "valkey-cli", "flushall")
            return migrate()
        time.sleep(1)
    raise SystemExit("Postgres did not become ready")
