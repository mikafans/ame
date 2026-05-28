/**
 * Formats a score (0.0 to 1.0) as a percentage string (e.g., "85%").
 */
export function formatScore(score: number): string {
  return `${Math.round((score ?? 0) * 100)}%`;
}

/**
 * Formats hours into a human-friendly duration string.
 * < 1m
 * Xm (if < 1h)
 * Xh Ym (if >= 1h)
 */
export function formatDuration(hours: number): string {
  if (hours < 1 / 60) return "< 1m";

  const totalMinutes = Math.round(hours * 60);
  if (totalMinutes < 60) {
    return `${totalMinutes}m`;
  }

  const h = Math.floor(totalMinutes / 60);
  const m = totalMinutes % 60;

  if (m === 0) {
    return `${h}h`;
  }
  return `${h}h ${m}m`;
}

/**
 * Formats minutes into a human-friendly duration string.
 */
export function formatMinutes(minutes: number): string {
  return formatDuration(minutes / 60);
}
