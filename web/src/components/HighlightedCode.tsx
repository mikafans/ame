"use client";

import { useEffect, useRef } from "react";
import Prism from "prismjs";
// Import language components
import "prismjs/components/prism-python";
import "prismjs/components/prism-javascript";
import "prismjs/components/prism-typescript";
import "prismjs/components/prism-bash";
import "prismjs/components/prism-json";
import "prismjs/components/prism-sql";
import Box from "@mui/material/Box";

interface Props {
  code: string;
  language?: string;
}

export function HighlightedCode({ code, language = "python" }: Props) {
  const codeRef = useRef<HTMLElement>(null);

  useEffect(() => {
    if (codeRef.current) {
      Prism.highlightElement(codeRef.current);
    }
  }, [code, language]);

  // Map common formats/names to prism languages
  const getPrismLanguage = (lang: string) => {
    const l = lang.toLowerCase().trim();
    if (l === "py" || l === "python") return "python";
    if (l === "js" || l === "javascript") return "javascript";
    if (l === "ts" || l === "typescript") return "typescript";
    if (l === "sh" || l === "bash" || l === "shell") return "bash";
    if (l === "json") return "json";
    if (l === "sql") return "sql";
    return "clike"; // default fallback
  };

  const prismLang = getPrismLanguage(language);
  const langClass = `language-${prismLang}`;

  return (
    <Box
      component="pre"
      className={langClass}
      sx={{
        margin: "0 !important",
        padding: "16px !important",
        borderRadius: "8px !important",
        fontSize: "13px !important",
        fontFamily: "monospace !important",
        overflowX: "auto",
        maxWidth: "100% !important",
        boxSizing: "border-box !important",
        backgroundColor: (theme) =>
          (theme.palette.mode === "dark" ? "#282c34" : "#fafafa") +
          " !important",
        color: (theme) =>
          (theme.palette.mode === "dark" ? "#abb2bf" : "#383a42") +
          " !important",
        border: "1px solid",
        borderColor: (theme) =>
          theme.palette.mode === "dark"
            ? "rgba(255,255,255,0.08)"
            : "rgba(0,0,0,0.08)",
        lineHeight: "1.5 !important",
        tabSize: "4 !important",
        "& code": {
          backgroundColor: "transparent !important",
          padding: "0 !important",
          borderRadius: "0 !important",
          fontFamily: "inherit !important",
          color: "inherit !important",
          whiteSpace: "pre !important",
          wordSpacing: "normal !important",
          wordBreak: "normal !important",
          wordWrap: "normal !important",
        },
        // Atom One Dark & Atom One Light Prism Token Colors
        "& .token.comment, & .token.prolog, & .token.doctype, & .token.cdata": {
          color: (theme) =>
            (theme.palette.mode === "dark" ? "#5c6370" : "#a0a1a7") +
            " !important",
          fontStyle: "italic !important",
        },
        "& .token.punctuation": {
          color: (theme) =>
            (theme.palette.mode === "dark" ? "#abb2bf" : "#383a42") +
            " !important",
        },
        "& .token.property, & .token.tag, & .token.boolean, & .token.number, & .token.constant, & .token.symbol, & .token.deleted":
          {
            color: (theme) =>
              (theme.palette.mode === "dark" ? "#d19a66" : "#986801") +
              " !important",
          },
        "& .token.selector, & .token.attr-name, & .token.string, & .token.char, & .token.builtin, & .token.inserted":
          {
            color: (theme) =>
              (theme.palette.mode === "dark" ? "#98c379" : "#50a14f") +
              " !important",
          },
        "& .token.operator, & .token.entity, & .token.url, & .language-css .token.string, & .style .token.string":
          {
            color: (theme) =>
              (theme.palette.mode === "dark" ? "#56b6c2" : "#0184bc") +
              " !important",
          },
        "& .token.atrule, & .token.attr-value, & .token.keyword": {
          color: (theme) =>
            (theme.palette.mode === "dark" ? "#c678dd" : "#a626a4") +
            " !important",
        },
        "& .token.function, & .token.class-name": {
          color: (theme) =>
            (theme.palette.mode === "dark" ? "#61afef" : "#4078f2") +
            " !important",
        },
        "& .token.regex, & .token.important, & .token.variable": {
          color: (theme) =>
            (theme.palette.mode === "dark" ? "#e06c75" : "#e45649") +
            " !important",
        },
        "& .token.important, & .token.bold": {
          fontWeight: "bold !important",
        },
        "& .token.italic": {
          fontStyle: "italic !important",
        },
      }}
    >
      <code ref={codeRef} className={langClass}>
        {code.trim()}
      </code>
    </Box>
  );
}
