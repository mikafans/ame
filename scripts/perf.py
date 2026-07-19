"""Run the existing oha performance scenarios through uv."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
API_URL = f"http://{os.environ.get('API_HOST', 'localhost')}:{os.environ.get('API_PORT', '28080')}"

if len(sys.argv) != 2 or sys.argv[1] not in {"load", "soak"}:
    raise SystemExit("usage: uv run scripts/perf.py load|soak")

if sys.argv[1] == "load":
    command = ["bash", "api_tests/perf/answer_load.sh", API_URL]
else:
    command = ["bash", "api_tests/perf/duration_load.sh", os.environ.get("DURATION", "1m"), API_URL]
raise SystemExit(subprocess.run(command, cwd=ROOT, check=False).returncode)
