from __future__ import annotations

import os
import signal
import subprocess

from .common import API_HOST, API_PORT, ROOT, WEB_PORT, compose, mise, run
from .database import migrate


def dev() -> int:
    if compose("up", "-d") != 0 or migrate() != 0:
        return 1
    env = {**os.environ, "DATABASE_URL": "postgres://postgres:postgres@localhost:5432/ame", "AME_PORT": API_PORT, "AME_CORS_ORIGINS": f"http://{API_HOST}:{WEB_PORT}", "AME_CONFIG_PATH": "ame.dev.toml", "RUST_LOG": "ame_api=debug,tower_http=info,sqlx=warn"}
    api = subprocess.Popen(["mise", "exec", "--", "cargo", "run", "--manifest-path", "api/Cargo.toml", "--bin", "ame-api"], cwd=ROOT, env=env, start_new_session=True)
    try:
        return mise("bun", "run", "dev", "--port", WEB_PORT, cwd=ROOT / "web", env={**env, "PORT": WEB_PORT, "NEXT_PUBLIC_API_URL": f"http://{API_HOST}:{API_PORT}"})
    finally:
        os.killpg(api.pid, signal.SIGTERM)
        api.wait(timeout=10)


def stop() -> int:
    run("fuser", "-k", "-9", f"{API_PORT}/tcp", f"{WEB_PORT}/tcp")
    return compose("down")


def init_env() -> int:
    commands = [("mise", "install", ROOT), ("cargo", "install", "sqlx-cli", "--no-default-features", "--features", "postgres", ROOT), ("bun", "install", ROOT / "web"), ("bunx", "playwright", "install", "--with-deps", ROOT / "web")]
    for *args, cwd in commands:
        status = mise(*args, cwd=cwd)
        if status != 0:
            return status
    return 0
