"use client";

import Button from "@mui/material/Button";
import Box from "@mui/material/Box";
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

  return (
    <div
      style={{
        background: "var(--surface-2)",
        border: "1px solid var(--border)",
        borderRadius: 6,
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
          justifyContent: "space-between",
          alignItems: "center",
        }}
      >
        <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
          <CodeOutlinedIcon sx={{ fontSize: 14, color: "text.secondary" }} />
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{ fontFamily: "monospace", letterSpacing: 0.8 }}
          >
            {filename} · {language}
          </Typography>
        </Box>
        <Box sx={{ display: "flex", gap: 0.75 }}>
          <Button size="small" variant="text">
            Run tests
          </Button>
          <Button size="small" variant="text">
            Reset
          </Button>
        </Box>
      </Box>

      {/* Code textarea */}
      <textarea
        value={current}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
        spellCheck={false}
        rows={10}
        style={{
          width: "100%",
          background: "transparent",
          color: "var(--text)",
          fontFamily: "var(--mono)",
          fontSize: 13.5,
          lineHeight: 1.6,
          padding: "16px 18px",
          border: "none",
          outline: "none",
          resize: "vertical",
          minHeight: 220,
          boxSizing: "border-box",
        }}
      />

      {/* Test output */}
      <div
        style={{
          padding: "10px 14px",
          borderTop: "1px solid var(--border)",
          background: "var(--surface)",
        }}
      >
        <div
          style={{
            fontFamily: "var(--mono)",
            fontSize: 11,
            color: "var(--muted)",
            marginBottom: 4,
            letterSpacing: 0.8,
          }}
        >
          ◇ Test runner output
        </div>
        <div
          style={{
            fontFamily: "var(--mono)",
            fontSize: 12,
            color: "var(--text-2)",
          }}
        >
          <span style={{ color: "var(--accent)" }}>✓</span> case 1: small graph
          — passed {"  "}
          <span style={{ color: "var(--accent)" }}>✓</span> case 2: disconnected
          — passed {"  "}
          <span style={{ color: "var(--red)" }}>✗</span> case 3: cycle — failed
        </div>
      </div>
    </div>
  );
}
