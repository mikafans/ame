"""Resolve a usable local container engine instead of trusting PATH alone."""

from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path


def engine() -> list[str]:
    configured = os.environ.get("COMPOSE")
    if configured:
        return configured.split()

    failures: list[str] = []
    for candidate in (("podman", "compose"), ("docker", "compose")):
        if shutil.which(candidate[0]) is None:
            continue
        probe = subprocess.run(
            [candidate[0], "info"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            text=True,
            check=False,
        )
        if probe.returncode == 0:
            return list(candidate)
        failures.append(f"{' '.join(candidate)}: {probe.stderr.strip().splitlines()[-1]}")

    detail = "\n".join(failures) or "no podman or docker executable found"
    raise SystemExit(
        "No usable container engine is available.\n"
        f"{detail}\n"
        "If Podman is installed, repair its machine with "
        "`podman machine stop && podman machine start`, or set COMPOSE to a working engine."
    )


def compose(file: Path, *args: str) -> subprocess.CompletedProcess[str]:
    cwd = file.parent.parent if file.parent.name == "db" else file.parent
    return subprocess.run(
        [*engine(), "-f", str(file), *args],
        cwd=cwd,
        text=True,
        check=True,
    )
