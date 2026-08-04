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
 * `message` is generic for validation failures.
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

  return message || fallback;
}

/**
 * Add status-aware guidance to an API error. This keeps 4xx/5xx handling
 * consistent across pages while preserving the server's useful validation
 * message and the rate-limit retry window.
 */
export function responseErrorMessage(
  response: Response,
  error: unknown,
  fallback: string,
): string {
  const message = errorMessage(error, fallback);
  if (response.status === 401) {
    return "Your session has expired. Sign in again to continue.";
  }
  if (response.status === 403) {
    return "You do not have permission to do that.";
  }
  if (response.status === 404) {
    return "That item is no longer available. Refresh and try again.";
  }
  if (response.status === 409) {
    return `${message} Refresh the page to reconcile the latest state.`;
  }
  if (response.status === 429) {
    const retryAfter = response.headers.get("retry-after");
    return retryAfter
      ? `${message} Please wait ${retryAfter} seconds and try again.`
      : `${message} Please wait a moment and try again.`;
  }
  if (response.status >= 500) {
    return "AME is having trouble completing that request. Your work is safe; try again shortly.";
  }
  return message;
}
