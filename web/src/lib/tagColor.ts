// Deterministic hue from a string (same name → same hue, no persistence needed).
function nameToHue(name: string): number {
  let hash = 0;
  for (let i = 0; i < name.length; i++) {
    hash = (hash * 31 + name.charCodeAt(i)) | 0;
  }
  return Math.abs(hash) % 360;
}

/**
 * Returns sx-ready color tokens for a tag chip.
 *
 * Light mode: muted, desaturated — readable dark text on a very pale background.
 * Dark  mode: vivid but not blinding — translucent fill with a bright border.
 *
 * @param name     Tag label (drives the hue).
 * @param isDark   Pass `mode === "dark"` from useColorMode().
 */
export function tagColor(
  name: string,
  isDark: boolean = false,
): { color: string; borderColor: string; bgcolor: string } {
  const h = nameToHue(name);
  if (isDark) {
    return {
      color: `hsl(${h} 75% 72%)`,
      borderColor: `hsl(${h} 60% 55% / 0.45)`,
      bgcolor: `hsl(${h} 65% 50% / 0.12)`,
    };
  }
  // Light: low saturation, dark text, very pale fill — no eye-scorching
  return {
    color: `hsl(${h} 50% 28%)`,
    borderColor: `hsl(${h} 40% 60% / 0.5)`,
    bgcolor: `hsl(${h} 45% 93%)`,
  };
}

/**
 * @deprecated Use tagColor(name, isDark) instead.
 */
export function vibrantTagColor(
  name: string,
  isDark: boolean = false,
): { color: string; borderColor: string; bgcolor: string } {
  return tagColor(name, isDark);
}
