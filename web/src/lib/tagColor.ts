// Deterministic color per tag name — same tag always gets the same hue, no
// persistence needed. Returns sx-ready color tokens that read well in both
// light and dark themes (saturated text/border over a faint translucent fill).
export function tagColor(name: string): {
  color: string;
  borderColor: string;
  bgcolor: string;
} {
  let hash = 0;
  for (let i = 0; i < name.length; i++) {
    hash = (hash * 31 + name.charCodeAt(i)) | 0;
  }
  const hue = Math.abs(hash) % 360;
  return {
    color: `hsl(${hue} 70% 45%)`,
    borderColor: `hsl(${hue} 60% 55% / 0.5)`,
    bgcolor: `hsl(${hue} 70% 50% / 0.1)`,
  };
}
