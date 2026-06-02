/**
 * Formats a score (0.0 to 1.0) as a percentage string (e.g., "85%").
 */
export function formatScore(score: number): string {
  return `${Math.round((score ?? 0) * 100)}%`;
}

type DateInput = string | number | Date;

function pad(n: number): string {
  return String(n).padStart(2, "0");
}

/**
 * Formats a date as `YYYY/MM/DD` in the viewer's local timezone.
 * Returns "" for missing/invalid input. Locale-independent by design so the
 * whole app shows one consistent date format.
 */
export function formatDate(input: DateInput | null | undefined): string {
  if (input == null) return "";
  const d = new Date(input);
  if (Number.isNaN(d.getTime())) return "";
  return `${d.getFullYear()}/${pad(d.getMonth() + 1)}/${pad(d.getDate())}`;
}

/**
 * Formats a time as 24-hour `HH:MM` in the viewer's local timezone.
 */
export function formatTime(input: DateInput | null | undefined): string {
  if (input == null) return "";
  const d = new Date(input);
  if (Number.isNaN(d.getTime())) return "";
  return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/**
 * Formats a date+time as `YYYY/MM/DD HH:MM` (24-hour) in local time.
 */
export function formatDateTime(input: DateInput | null | undefined): string {
  const date = formatDate(input);
  return date ? `${date} ${formatTime(input)}` : "";
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
