# Public agent entry contract

The generated machine manifest and the concise `llms.txt` entry document now
describe the same current 0.3 API path set. The entry document explicitly
includes the complete learning loop: onboarding, journey/session work,
question and assessment attempts, evidence and mastery, recommendations,
streak writes/reads, timeline, deep dives, and generation runs.

The contract is enforced by
`api_tests/test_contracts.py::test_llms_entry_doc_lists_every_manifest_api_path`.
It derives paths from `docs/public/skill.json` and fails if any path is absent
from `docs/public/llms.txt`; query strings are ignored for this comparison.

Static public files remain Caddy-served at `/public/*`; the listed learner
resources remain API-backed at `/api/v1/*`. No route, schema migration, or
second agent identity was added.
