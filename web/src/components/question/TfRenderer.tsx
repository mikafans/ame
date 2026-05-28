"use client";

import Box from "@mui/material/Box";
import Typography from "@mui/material/Typography";
import CheckCircleOutlineIcon from "@mui/icons-material/CheckCircleOutline";
import CancelOutlinedIcon from "@mui/icons-material/CancelOutlined";
import { useTheme } from "@mui/material/styles";

interface Props {
  value: boolean | null;
  onChange: (v: boolean) => void;
  disabled?: boolean;
}

const OPTIONS = [
  {
    v: true,
    label: "True",
    Icon: CheckCircleOutlineIcon,
    color: "success" as const,
  },
  {
    v: false,
    label: "False",
    Icon: CancelOutlinedIcon,
    color: "error" as const,
  },
];

export function TfRenderer({ value, onChange, disabled = false }: Props) {
  const theme = useTheme();

  return (
    <Box sx={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 2 }}>
      {OPTIONS.map(({ v, label, Icon, color }) => {
        const selected = value === v;
        const palette = theme.palette[color];
        return (
          <Box
            key={label}
            onClick={() => !disabled && onChange(v)}
            sx={{
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
              gap: 1.5,
              py: 4,
              border: `2px solid ${selected ? palette.main : theme.palette.divider}`,
              borderRadius: 2,
              bgcolor: selected ? `${color}.50` : "background.paper",
              cursor: disabled ? "default" : "pointer",
              transition: "border-color 0.15s, background-color 0.15s",
              "&:hover": disabled
                ? {}
                : {
                    borderColor: selected ? palette.main : palette.light,
                    bgcolor: selected ? `${color}.50` : `${color}.50`,
                  },
            }}
          >
            <Icon
              sx={{
                fontSize: 36,
                color: selected ? palette.main : "text.disabled",
                transition: "color 0.15s",
              }}
            />
            <Typography
              variant="h6"
              sx={{
                fontWeight: 600,
                color: selected ? palette.main : "text.secondary",
              }}
            >
              {label}
            </Typography>
          </Box>
        );
      })}
    </Box>
  );
}
