# AME Python SDK

This is the reusable Python client for AME's public learning contract. The
same `ame.py` file is served by every AME API at:

```text
GET /public/sdk/python/ame.py
```

Download and run it without cloning AME:

```bash
curl -fsS https://ame.example.com/public/sdk/python/ame.py -o ame.py
python ame.py --base-url https://ame.example.com discover
```

Or install the package from this directory and use the typed convenience
methods plus `client.request(...)` for any operation in `skill.json`:

```bash
pip install .
python - <<'PY'
from ame import AmeClient

with AmeClient("https://ame.example.com") as client:
    print(client.discover())
PY
```

For a delegated handoff, `AmeClient.from_handoff(handoff_text)` extracts the
temporary bearer capability and public origin without hardcoding a host.
The downloaded module uses only Python's standard library, so no dependency is
needed for the direct `python ame.py ...` workflow.
