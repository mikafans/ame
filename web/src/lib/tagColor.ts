import { BRAND_TAG_HUES } from "@/lib/brand";

// Deterministic hue from a string (same name → same hue, no persistence needed).
function nameToPaletteHue(name: string): number {
  let hash = 0;
  for (let i = 0; i < name.length; i++) {
    hash = (hash * 31 + name.charCodeAt(i)) | 0;
  }
  return BRAND_TAG_HUES[Math.abs(hash) % BRAND_TAG_HUES.length];
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
  const h = nameToPaletteHue(name);
  if (isDark) {
    return {
      color: `hsl(${h} 72% 74%)`,
      borderColor: `hsl(${h} 58% 58% / 0.42)`,
      bgcolor: `hsl(${h} 62% 48% / 0.11)`,
    };
  }
  // Light: low saturation, dark text, very pale fill — no eye-scorching
  return {
    color: `hsl(${h} 48% 27%)`,
    borderColor: `hsl(${h} 36% 62% / 0.45)`,
    bgcolor: `hsl(${h} 42% 94%)`,
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
