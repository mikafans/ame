from __future__ import annotations

import os

from .common import ROOT, tool, web_ready


def format_project(check: bool) -> int:
    cargo = ("cargo", "fmt", "--check") if check else ("cargo", "fmt")
    status = tool(*cargo, cwd=ROOT / "api")
    if status != 0:
        return status
    if web_ready():
        prettier = ("bunx", "prettier", "--check", ".") if check else ("bunx", "prettier", "--write", ".")
        status = tool(*prettier, cwd=ROOT / "web")
        if status != 0:
            return status
    return tool("uvx", "sqlfluff", "lint" if check else "format", "db/migrations")


def lint() -> int:
    status = tool("cargo", "clippy", "--all-targets", "--", "-D", "warnings", cwd=ROOT / "api")
    if status != 0:
        return status
    if web_ready():
        for command in (("bun", "run", "type-check"), ("bun", "run", "api:check")):
            status = tool(*command, cwd=ROOT / "web")
            if status != 0:
                return status
    return tool("uvx", "sqlfluff", "lint", "db/migrations")


def test() -> int:
    status = tool("cargo", "test", cwd=ROOT / "api")
    if status == 0 and web_ready():
        status = tool("bun", "test", "src/", cwd=ROOT / "web")
    return status


DB_TESTS = ["login_sessions", "agent_manifest"]


def test_db(name: str | None = None) -> int:
    selected = [name] if name else DB_TESTS
    env = {**os.environ, "AME_RUN_DB_TESTS": "1", "AME_CONFIG_PATH": "../ame.dev.toml"}
    for test_name in selected:
        args = ["cargo", "test", "--test", test_name, "--", "--nocapture"]
        status = tool(*args, cwd=ROOT / "api", env=env)
        if status != 0:
            return status
    return 0
