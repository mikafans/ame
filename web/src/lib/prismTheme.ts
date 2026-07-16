/** Map common language names and aliases to a Prism grammar key. */
export function toPrismLanguage(lang: string): string {
  const l = lang.toLowerCase().trim();
  if (l === "py" || l === "python") return "python";
  if (l === "js" || l === "javascript") return "javascript";
  if (l === "ts" || l === "typescript") return "typescript";
  if (l === "sh" || l === "bash" || l === "shell") return "bash";
  if (["json", "sql"].includes(l)) return l;
  return "clike";
}
