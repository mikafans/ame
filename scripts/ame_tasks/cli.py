from __future__ import annotations

import argparse

from container_runtime import engine

from .common import PREVIEW_PORT, ROOT, run, tool, web_ready
from .database import down, migrate, reset, up
from .quality import format_project, lint, test, test_db
from .runtime import init_env


def dispatch(target: str) -> int:
    if target == "fmt":
        return format_project(False)
    if target == "fmt-check":
        return format_project(True)
    if target == "lint":
        return lint()
    if target == "test":
        return test()
    if target == "test-db":
        return test_db()
    if target == "db-up":
        return up()
    if target == "db-down":
        return down()
    if target == "db-reset":
        return reset()
    if target == "db-migrate":
        return migrate()
    if target == "openapi":
        status = tool("cargo", "run", "--quiet", "--bin", "gen-openapi", cwd=ROOT / "api")
        if status != 0 or not web_ready():
            return status
        status = tool("bun", "run", "api:gen", cwd=ROOT / "web")
        if status != 0:
            return status
        return 0
    if target == "public-docs":
        return tool("cargo", "run", "--quiet", "--bin", "gen-public-docs", cwd=ROOT / "api")
    if target == "init-env":
        return init_env()
    if target == "preview":
        return tool(
            "uv",
            "run",
            "--no-project",
            "python",
            "-m",
            "http.server",
            PREVIEW_PORT,
            "--bind",
            "0.0.0.0",
            "--directory",
            "design/preview",
        )
    if target.startswith("docker-"):
        action = target.removeprefix("docker-")
        args = (*engine(), "-f", "deploy/docker-compose.prod.yml", action)
        if action == "up":
            args += ("-d",)
        if action == "logs":
            args += ("-f",)
        return run(*args)
    raise SystemExit(f"unknown task: {target}")


def main() -> int:
    parser = argparse.ArgumentParser(description="AME project task runner")
    parser.add_argument("target")
    return dispatch(parser.parse_args().target)


if __name__ == "__main__":
    raise SystemExit(main())
