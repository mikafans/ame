"use client";

import Box from "@mui/material/Box";
import TextField from "@mui/material/TextField";
import Typography from "@mui/material/Typography";
import CodeOutlinedIcon from "@mui/icons-material/CodeOutlined";

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

  const INDENT = "  ";

  // Tab/Shift+Tab indent inside the editor instead of moving focus. This traps
  // Tab while focused — acceptable for a code field; Esc still blurs it.
  function handleKeyDown(e: React.KeyboardEvent<HTMLDivElement>) {
    if (e.key !== "Tab" || disabled) return;
    e.preventDefault();
    const ta = e.target as HTMLTextAreaElement;
    const start = ta.selectionStart;
    const end = ta.selectionEnd;

    if (e.shiftKey) {
      const lineStart = current.lastIndexOf("\n", start - 1) + 1;
      const removed =
        current.slice(lineStart).match(/^ {1,2}/)?.[0].length ?? 0;
      if (removed === 0) return;
      const next =
        current.slice(0, lineStart) + current.slice(lineStart + removed);
      onChange(next);
      requestAnimationFrame(() => {
        const caret = Math.max(lineStart, start - removed);
        ta.selectionStart = ta.selectionEnd = caret;
      });
    } else {
      const next = current.slice(0, start) + INDENT + current.slice(end);
      onChange(next);
      requestAnimationFrame(() => {
        ta.selectionStart = ta.selectionEnd = start + INDENT.length;
      });
    }
  }

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

      {/* Code editor */}
      <TextField
        fullWidth
        multiline
        minRows={10}
        value={current}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={handleKeyDown}
        disabled={disabled}
        placeholder="Write your solution here…"
        spellCheck={false}
        variant="standard"
        slotProps={{ input: { disableUnderline: true } }}
        sx={{
          "& .MuiInputBase-root": {
            fontFamily: "monospace",
            fontSize: 13.5,
            lineHeight: 1.6,
            p: "16px 18px",
            alignItems: "flex-start",
          },
        }}
      />
    </Box>
  );
}
