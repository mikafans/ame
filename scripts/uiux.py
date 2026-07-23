"""Run the focused UI/UX contract against the host development ports."""

from __future__ import annotations

import os
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
API_HOST = os.environ.get("API_HOST", "localhost")
API_PORT = os.environ.get("API_PORT", "28080")
WEB_PORT = os.environ.get("WEB_PORT", "23000")

env = {
    **os.environ,
    "PORT": WEB_PORT,
    "NEXT_PUBLIC_API_URL": f"http://{API_HOST}:{API_PORT}",
    "E2E_API_URL": f"http://{API_HOST}:{API_PORT}",
    "E2E_BASE_URL": f"http://{API_HOST}:{WEB_PORT}",
}
raise SystemExit(
    subprocess.run(
        [
            "bunx",
            "playwright",
            "test",
            "e2e/uiux.spec.ts",
            "--project=chromium",
        ],
        cwd=ROOT / "web",
        env=env,
        check=False,
    ).returncode
)
