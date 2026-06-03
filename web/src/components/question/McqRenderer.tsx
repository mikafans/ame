"use client";

import Box from "@mui/material/Box";
import Typography from "@mui/material/Typography";
import { useTheme } from "@mui/material/styles";

interface McqOption {
  text: string;
  index: number;
}

interface Props {
  options: McqOption[];
  value: number | null;
  onChange: (position: number) => void;
  disabled?: boolean;
}

const LABELS = ["A", "B", "C", "D", "E", "F"];

export function McqRenderer({
  options,
  value,
  onChange,
  disabled = false,
}: Props) {
  const theme = useTheme();

  return (
    <Box sx={{ display: "flex", flexDirection: "column" }}>
      {options.map((opt, displayIdx) => {
        const selected = value === opt.index;
        return (
          <Box
            key={opt.index}
            role="button"
            tabIndex={disabled ? -1 : 0}
            onClick={() => !disabled && onChange(opt.index)}
            onKeyDown={(e) =>
              e.key === "Enter" || e.key === " "
                ? !disabled && onChange(opt.index)
                : undefined
            }
            sx={{
              display: "flex",
              alignItems: "center",
              gap: 2,
              px: 2.5,
              py: 1.75,
              borderTop:
                displayIdx === 0
                  ? `1px solid ${theme.palette.divider}`
                  : "none",
              borderBottom: `1px solid ${theme.palette.divider}`,
              borderLeft: `3px solid ${selected ? theme.palette.primary.main : "transparent"}`,
              bgcolor: selected
                ? theme.palette.mode === "dark"
                  ? "rgba(25, 118, 210, 0.16)"
                  : "rgba(25, 118, 210, 0.08)"
                : "background.paper",
              cursor: disabled ? "default" : "pointer",
              transition: "background-color 0.15s, border-left-color 0.15s",
              "&:hover": disabled
                ? {}
                : {
                    bgcolor: selected
                      ? theme.palette.mode === "dark"
                        ? "rgba(25, 118, 210, 0.2)"
                        : "rgba(25, 118, 210, 0.12)"
                      : "action.hover",
                  },
            }}
          >
            <Box
              sx={{
                flexShrink: 0,
                width: 28,
                height: 28,
                borderRadius: "50%",
                border: `2px solid ${selected ? theme.palette.primary.main : theme.palette.divider}`,
                bgcolor: selected ? "primary.main" : "transparent",
                color: selected ? "primary.contrastText" : "text.secondary",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                fontSize: 12,
                fontWeight: 700,
              }}
            >
              {LABELS[displayIdx] ?? displayIdx + 1}
            </Box>

            <Typography
              variant="body1"
              sx={{
                color: selected ? "primary.main" : "text.primary",
                lineHeight: 1.5,
              }}
            >
              {opt.text}
            </Typography>
          </Box>
        );
      })}
    </Box>
  );
}
