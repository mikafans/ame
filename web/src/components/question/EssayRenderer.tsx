"use client";

import Box from "@mui/material/Box";
import TextField from "@mui/material/TextField";
import Typography from "@mui/material/Typography";

interface Props {
  value: string;
  onChange: (v: string) => void;
  disabled?: boolean;
}

export function EssayRenderer({ value, onChange, disabled = false }: Props) {
  const wordCount = value.trim() ? value.trim().split(/\s+/).length : 0;

  return (
    <Box>
      <TextField
        fullWidth
        multiline
        minRows={8}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
        placeholder="Write your answer here…"
      />
      <Typography
        variant="caption"
        color="text.secondary"
        sx={{ display: "block", textAlign: "right", mt: 0.5 }}
      >
        {wordCount} word{wordCount !== 1 ? "s" : ""}
      </Typography>
    </Box>
  );
}
