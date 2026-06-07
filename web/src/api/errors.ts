/**
 * Shared API error handling.
 *
 * The backend serializes every error through one envelope (see
 * `api/src/domain/error.rs`): `{ error: { code, message, details? } }`.
 * openapi-fetch surfaces that parsed body as the `error` half of its
 * `{ data, error }` result, so the useful text lives at `error.error.message`
 * — NOT `error.message`. Reading the wrong path silently drops the backend's
 * message and falls back to a generic string, which is the bug class this
 * module exists to prevent. Always route API errors through `errorMessage`.
 */

interface ApiErrorBody {
  error?: {
    code?: string;
    message?: string;
    details?: Record<string, unknown>;
  };
  // Some non-enveloped failures (network shims) put message at the top level.
  message?: string;
}

/**
 * Extract a human-readable message from an openapi-fetch `error` value,
 * unpacking the structured `details` for the cases where the top-level
 * `message` is generic (validation, quota).
 */
export function errorMessage(error: unknown, fallback: string): string {
  const body = error as ApiErrorBody;
  const env = body?.error;
  const message = env?.message ?? body?.message;

  // validation_failed carries a generic top-level message; the useful detail is
  // in details.fields[].message — surface those instead.
  if (env?.code === "validation_failed" && env.details) {
    const fields = (env.details as { fields?: Array<{ message?: string }> })
      .fields;
    const reasons = fields?.map((f) => f.message).filter(Boolean);
    if (reasons && reasons.length > 0) return reasons.join("; ");
  }

  // quota_exceeded carries the limit/usage worth showing alongside the message.
  if (env?.code === "quota_exceeded" && env.details) {
    const { limit, usage } = env.details as { limit?: number; usage?: number };
    if (limit != null) {
      return `${message ?? fallback} (using ${usage ?? "?"} of ${limit}). Upgrade to premium for a higher limit.`;
    }
  }

  return message || fallback;
}
