from __future__ import annotations

from .common import ROOT, tool


def init_env() -> int:
    # Language runtimes and sqlx-cli come from the nix devShell; init-env only
    # installs project-local deps that nix does not manage.
    commands = [("bun", "install", ROOT / "web"), ("bunx", "playwright", "install", "--with-deps", ROOT / "web")]
    for *args, cwd in commands:
        status = tool(*args, cwd=cwd)
        if status != 0:
            return status
    return 0
