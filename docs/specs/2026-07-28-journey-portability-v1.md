# Journey portability v1

`ame.journey-history.v1` is AME's owner-scoped portability contract. It is an
application manifest, not a PostgreSQL backup.

The envelope contains `schemaVersion`, `ownerId`, `journeyId`, `exportedAt`, a
SHA-256 `checksum`, and `payload`. The checksum covers only the canonical JSON
payload so re-export time does not change artifact identity. Payload arrays
contain the journey curriculum, immutable source and citation provenance,
progress evidence, review scheduling, private notes, task revisions, and an
append-only history archive for sessions and assessment attempts.

Imports require the authenticated owner to match `ownerId`. AME rejects unknown
schema versions, missing arrays, checksum tampering, foreign-owner rows, and
conflicting records. A repeated checksum returns the original receipt without
duplicating state. Import never creates, merges, or changes an account identity.

Reviewer IDs from another installation remain in the portable artifact and its
review provenance. If that reviewer identity is not configured locally, the
operational foreign key is left empty while the archived manifest preserves the
original attribution.
