from __future__ import annotations

from .common import ROOT, mise, web_ready


def format_project(check: bool) -> int:
    cargo = ("cargo", "fmt", "--check") if check else ("cargo", "fmt")
    status = mise(*cargo, cwd=ROOT / "api")
    if status != 0:
        return status
    if web_ready():
        prettier = ("bunx", "prettier", "--check", ".") if check else ("bunx", "prettier", "--write", ".")
        status = mise(*prettier, cwd=ROOT / "web")
        if status != 0:
            return status
    return mise("uvx", "sqlfluff", "lint" if check else "format", "db/migrations")


def lint() -> int:
    status = mise("cargo", "clippy", "--all-targets", "--", "-D", "warnings", cwd=ROOT / "api")
    if status != 0:
        return status
    if web_ready():
        for command in (("bun", "run", "type-check"), ("bun", "run", "api:check"), ("bun", "run", "client:check")):
            status = mise(*command, cwd=ROOT / "web")
            if status != 0:
                return status
    return mise("uvx", "sqlfluff", "lint", "db/migrations")


def test() -> int:
    status = mise("cargo", "test", cwd=ROOT / "api")
    if status == 0 and web_ready():
        status = mise("bun", "test", "src/", cwd=ROOT / "web")
    return status


DB_TESTS = ["auth", "login_sessions", "bank", "me", "agent_tools", "cross_owner", "sessions", "exams", "quota", "stats", "planner", "agent_manifest", "export", "admin", "admin_settings", "retention"]


def test_db(name: str | None = None) -> int:
    selected = [name] if name else DB_TESTS
    env = {"AME_RUN_DB_TESTS": "1", "AME_CONFIG_PATH": "../ame.dev.toml"}
    for test_name in selected:
        args = ["cargo", "test", "--test", test_name, "--", "--nocapture"]
        if test_name in {"admin_settings", "retention"}:
            args.append("--test-threads=1")
        status = mise(*args, cwd=ROOT / "api", env=env)
        if status != 0:
            return status
    return 0


def test_engine() -> int:
    return mise("cargo", "test", "engine", cwd=ROOT / "api") or mise("cargo", "test", "--test", "planner", cwd=ROOT / "api")
