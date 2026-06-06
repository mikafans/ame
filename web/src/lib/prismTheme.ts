import type { Theme } from "@mui/material/styles";
import type { SystemStyleObject } from "@mui/system";

/**
 * Atom One (light + dark) Prism token colors, shared by the read-only
 * results view (HighlightedCode) and the in-session code editor (CodeRenderer)
 * so highlighting is identical in both places. Spread into a component's `sx`.
 * Typed as a plain style object (not `SxProps`) so it can be spread into an
 * `sx` object literal without tripping the array/function union.
 */
export const prismTokenSx: SystemStyleObject<Theme> = {
  "& .token.comment, & .token.prolog, & .token.doctype, & .token.cdata": {
    color: (theme) =>
      (theme.palette.mode === "dark" ? "#5c6370" : "#a0a1a7") + " !important",
    fontStyle: "italic !important",
  },
  "& .token.punctuation": {
    color: (theme) =>
      (theme.palette.mode === "dark" ? "#abb2bf" : "#383a42") + " !important",
  },
  "& .token.property, & .token.tag, & .token.boolean, & .token.number, & .token.constant, & .token.symbol, & .token.deleted":
    {
      color: (theme) =>
        (theme.palette.mode === "dark" ? "#d19a66" : "#986801") + " !important",
    },
  "& .token.selector, & .token.attr-name, & .token.string, & .token.char, & .token.builtin, & .token.inserted":
    {
      color: (theme) =>
        (theme.palette.mode === "dark" ? "#98c379" : "#50a14f") + " !important",
    },
  "& .token.operator, & .token.entity, & .token.url, & .language-css .token.string, & .style .token.string":
    {
      color: (theme) =>
        (theme.palette.mode === "dark" ? "#56b6c2" : "#0184bc") + " !important",
    },
  "& .token.atrule, & .token.attr-value, & .token.keyword": {
    color: (theme) =>
      (theme.palette.mode === "dark" ? "#c678dd" : "#a626a4") + " !important",
  },
  "& .token.function, & .token.class-name": {
    color: (theme) =>
      (theme.palette.mode === "dark" ? "#61afef" : "#4078f2") + " !important",
  },
  "& .token.regex, & .token.important, & .token.variable": {
    color: (theme) =>
      (theme.palette.mode === "dark" ? "#e06c75" : "#e45649") + " !important",
  },
  "& .token.important, & .token.bold": {
    fontWeight: "bold !important",
  },
  "& .token.italic": {
    fontStyle: "italic !important",
  },
};

/** Map common language names/aliases to a Prism grammar key. */
export function toPrismLanguage(lang: string): string {
  const l = lang.toLowerCase().trim();
  if (l === "py" || l === "python") return "python";
  if (l === "js" || l === "javascript") return "javascript";
  if (l === "ts" || l === "typescript") return "typescript";
  if (l === "sh" || l === "bash" || l === "shell") return "bash";
  if (l === "json") return "json";
  if (l === "sql") return "sql";
  return "clike"; // default fallback
}
