"use client";

import Box from "@mui/material/Box";
import Typography from "@mui/material/Typography";
import { useColorMode } from "@/components/ThemeRegistry";
import { BRAND_GRADIENT } from "@/lib/brand";

interface BrandMarkProps {
  size?: number;
}

export function BrandMark({ size = 32 }: BrandMarkProps) {
  const { mode } = useColorMode();
  const mascotSrc =
    mode === "dark" ? "/miku-icon-dark.svg" : "/miku-icon-light.svg";

  return (
    <Box
      data-testid="brand-mark"
      sx={{ display: "flex", alignItems: "center", gap: 1.25 }}
    >
      <Box
        component="img"
        src={mascotSrc}
        alt=""
        aria-hidden="true"
        data-testid="brand-mascot"
        sx={{
          width: size,
          height: size,
          display: "block",
          flex: "0 0 auto",
          borderRadius: 2,
        }}
      />
      <Typography
        component="span"
        data-testid="brand-wordmark"
        sx={{
          fontSize: Math.round(size * 0.75),
          fontWeight: 800,
          lineHeight: 1,
          letterSpacing: "-0.02em",
          background: BRAND_GRADIENT,
          backgroundClip: "text",
          WebkitBackgroundClip: "text",
          color: "transparent",
        }}
      >
        ame
      </Typography>
    </Box>
  );
}
