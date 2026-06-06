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
import { prismTokenSx, toPrismLanguage } from "@/lib/prismTheme";

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

  const prismLang = toPrismLanguage(language);
  const langClass = `language-${prismLang}`;

  return (
    <Box
      component="pre"
      className={langClass}
      sx={[
        {
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
        },
        // Atom One Dark & Atom One Light Prism token colors (shared with the
        // in-session editor via prismTokenSx).
        prismTokenSx,
      ]}
    >
      <code ref={codeRef} className={langClass}>
        {code.trim()}
      </code>
    </Box>
  );
}
