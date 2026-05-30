"use client";

import TextField from "@mui/material/TextField";

interface Props {
  value: string;
  onChange: (v: string) => void;
  disabled?: boolean;
}

export function ShortRenderer({ value, onChange, disabled = false }: Props) {
  return (
    <TextField
      fullWidth
      size="small"
      value={value}
      onChange={(e) => onChange(e.target.value)}
      disabled={disabled}
      placeholder="Your answer…"
    />
  );
}
