"use client";

import Box from "@mui/material/Box";
import Typography from "@mui/material/Typography";
import CodeOutlinedIcon from "@mui/icons-material/CodeOutlined";
import Editor from "react-simple-code-editor";
import Prism from "prismjs";
import "prismjs/components/prism-python";
import "prismjs/components/prism-javascript";
import "prismjs/components/prism-typescript";
import "prismjs/components/prism-bash";
import "prismjs/components/prism-json";
import "prismjs/components/prism-sql";
import { prismTokenSx, toPrismLanguage } from "@/lib/prismTheme";

interface Props {
  value: string;
  onChange: (v: string) => void;
  disabled?: boolean;
  language?: string;
  filename?: string;
  starter?: string;
}

export function CodeRenderer({
  value,
  onChange,
  disabled = false,
  language = "python",
  filename = "solution.py",
  starter = "",
}: Props) {
  const current = value || starter;
  const prismLang = toPrismLanguage(language);

  const highlight = (code: string) => {
    const grammar = Prism.languages[prismLang] ?? Prism.languages.clike;
    return Prism.highlight(code, grammar, prismLang);
  };

  return (
    <Box
      sx={{
        border: 1,
        borderColor: "divider",
        borderRadius: 1.5,
        overflow: "hidden",
      }}
    >
      {/* File header */}
      <Box
        sx={{
          px: 1.75,
          py: 1,
          borderBottom: 1,
          borderColor: "divider",
          bgcolor: "action.hover",
          display: "flex",
          alignItems: "center",
          gap: 1,
        }}
      >
        <CodeOutlinedIcon sx={{ fontSize: 14, color: "text.secondary" }} />
        <Typography
          variant="caption"
          color="text.secondary"
          sx={{ fontFamily: "monospace", letterSpacing: 0.8 }}
        >
          {filename} · {language}
        </Typography>
      </Box>

      {/* Code editor: transparent textarea over a Prism-highlighted layer,
          driven by react-simple-code-editor (handles Tab-indent + scroll sync). */}
      <Box
        sx={[
          {
            minHeight: 240,
            fontFamily: "monospace",
            fontSize: 13.5,
            lineHeight: 1.6,
            tabSize: 2,
            bgcolor: (theme) =>
              theme.palette.mode === "dark" ? "#282c34" : "#fafafa",
            color: (theme) =>
              theme.palette.mode === "dark" ? "#abb2bf" : "#383a42",
            "& textarea:focus": { outline: "none" },
            "& .token": { background: "transparent !important" },
          },
          prismTokenSx,
        ]}
      >
        <Editor
          value={current}
          onValueChange={disabled ? () => {} : onChange}
          highlight={highlight}
          disabled={disabled}
          tabSize={2}
          insertSpaces
          padding={16}
          placeholder="Write your solution here…"
          spellCheck={false}
          style={{
            fontFamily: "inherit",
            fontSize: "inherit",
            minHeight: 240,
          }}
        />
      </Box>
    </Box>
  );
}
