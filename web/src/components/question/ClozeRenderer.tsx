"use client";

import Box from "@mui/material/Box";
import TextField from "@mui/material/TextField";

interface Props {
  prompt: string;
  value: string;
  onChange: (v: string) => void;
  disabled?: boolean;
}

export function ClozeRenderer({
  prompt,
  value,
  onChange,
  disabled = false,
}: Props) {
  // Split prompt on ___ to render inline blanks
  const parts = prompt.split("___");

  if (parts.length <= 1) {
    return (
      <TextField
        fullWidth
        size="small"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
        placeholder="Fill in the blank…"
      />
    );
  }

  return (
    <Box sx={{ fontSize: 15, lineHeight: 2.4 }}>
      {parts.map((part, i) => (
        <span key={i}>
          {part}
          {i < parts.length - 1 && (
            <TextField
              variant="standard"
              value={value}
              onChange={(e) => onChange(e.target.value)}
              disabled={disabled}
              placeholder="___"
              sx={{
                width: 160,
                mx: 0.75,
                verticalAlign: "baseline",
              }}
              slotProps={{ htmlInput: { style: { textAlign: "center" } } }}
            />
          )}
        </span>
      ))}
    </Box>
  );
}
