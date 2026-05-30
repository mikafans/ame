"use client";

import Box from "@mui/material/Box";
import Typography from "@mui/material/Typography";
import CheckCircleOutlineIcon from "@mui/icons-material/CheckCircleOutline";

interface Props {
  items: string[];
  kicker?: string;
  compact?: boolean;
  accentBars?: boolean;
}

export function LearningObjectives({
  items,
  kicker = "What you'll learn",
  compact = false,
  accentBars = true,
}: Props) {
  if (!items.length) return null;

  const overLimit = items.length > 6;

  if (compact) {
    return (
      <Box>
        <Typography
          variant="caption"
          color="text.secondary"
          sx={{
            display: "block",
            letterSpacing: 1,
            textTransform: "uppercase",
            mb: 1,
          }}
        >
          {kicker}
        </Typography>
        <Box
          component="ul"
          sx={{
            listStyle: "none",
            p: 0,
            m: 0,
            display: "flex",
            flexDirection: "column",
            gap: 1,
          }}
        >
          {items.map((it, i) => (
            <Box
              component="li"
              key={i}
              sx={{ display: "flex", gap: 1, alignItems: "flex-start" }}
            >
              <CheckCircleOutlineIcon
                sx={{
                  fontSize: 16,
                  color: "primary.main",
                  mt: "2px",
                  flexShrink: 0,
                }}
              />
              <Typography variant="body2" color="text.secondary">
                {it}
              </Typography>
            </Box>
          ))}
        </Box>
        {overLimit && (
          <Typography
            variant="caption"
            color="warning.main"
            sx={{ display: "inline-block", mt: 1 }}
          >
            {items.length} objectives — consider splitting
          </Typography>
        )}
      </Box>
    );
  }

  return (
    <Box
      sx={{
        p: 2.5,
        bgcolor: "action.hover",
        border: 1,
        borderColor: "divider",
        borderRadius: 2,
      }}
    >
      <Box sx={{ display: "flex", alignItems: "center", gap: 1, mb: 1.5 }}>
        <Typography
          variant="caption"
          color="text.secondary"
          sx={{ letterSpacing: 1, textTransform: "uppercase" }}
        >
          {kicker}
        </Typography>
        {overLimit && (
          <Typography
            variant="caption"
            color="warning.main"
            sx={{ ml: "auto" }}
          >
            {items.length} objectives
          </Typography>
        )}
      </Box>
      <Box
        component="ul"
        sx={{
          listStyle: "none",
          p: 0,
          m: 0,
          display: "grid",
          gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr" },
          gap: 1.25,
        }}
      >
        {items.map((it, i) => (
          <Box
            component="li"
            key={i}
            sx={{
              display: "flex",
              gap: 1.25,
              alignItems: "flex-start",
              pl: accentBars ? 1.25 : 0,
              borderLeft: accentBars ? 2 : 0,
              borderColor: "primary.main",
            }}
          >
            <Typography
              variant="caption"
              color="text.secondary"
              sx={{ fontFamily: "monospace", pt: "2px", flexShrink: 0 }}
            >
              {String(i + 1).padStart(2, "0")}
            </Typography>
            <Typography variant="body2" color="text.secondary">
              {it}
            </Typography>
          </Box>
        ))}
      </Box>
    </Box>
  );
}
