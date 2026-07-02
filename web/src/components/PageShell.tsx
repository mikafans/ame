/**
 * PageShell — unified page layout wrapper used by all learner & admin pages.
 *
 * Provides:
 *  - Consistent horizontal/vertical padding (48px sides, 40px top, 64px bottom)
 *  - Standard kicker + title + optional subtitle header pattern
 *  - Optional right-hand action slot (e.g. a button)
 *  - Subtle fade-in-up entrance animation
 */
import Box from "@mui/material/Box";
import Typography from "@mui/material/Typography";
import React from "react";

interface PageShellProps {
  /** Small uppercase label above the title */
  kicker?: string;
  /** Main page title */
  title: string;
  /** Optional subtitle below the title */
  subtitle?: string;
  /** Optional element placed in the top-right of the header row */
  action?: React.ReactNode;
  /** Max width for the content area (default: 900) */
  maxWidth?: number | string;
  children: React.ReactNode;
}

export function PageShell({
  kicker,
  title,
  subtitle,
  action,
  maxWidth = "100%",
  children,
}: PageShellProps) {
  return (
    <Box
      sx={{
        px: { xs: 3, sm: 5 },
        pt: 5,
        pb: 8,
        maxWidth,
        mx: "auto", // Center the container horizontally
        "@keyframes fadeUp": {
          from: { opacity: 0, transform: "translateY(10px)" },
          to: { opacity: 1, transform: "translateY(0)" },
        },
        animation: "fadeUp 0.22s ease-out both",
      }}
    >
      {/* Header */}
      <Box
        sx={{
          display: "flex",
          flexDirection: { xs: "column", sm: "row" },
          gap: { xs: 2, sm: 0 },
          justifyContent: "space-between",
          alignItems: { xs: "flex-start", sm: "flex-start" },
          mb: 4,
        }}
      >
        <Box>
          {kicker && (
            <Typography
              variant="overline"
              color="text.disabled"
              sx={{ letterSpacing: 1.5, display: "block", mb: 0.25 }}
            >
              {kicker}
            </Typography>
          )}
          <Typography
            variant="h5"
            component="h1"
            sx={{ fontWeight: 600, lineHeight: 1.25 }}
          >
            {title}
          </Typography>
          {subtitle && (
            <Typography
              variant="body2"
              color="text.secondary"
              sx={{ mt: 0.5, maxWidth: 540 }}
            >
              {subtitle}
            </Typography>
          )}
        </Box>
        {action && (
          <Box
            sx={{
              flexShrink: 0,
              ml: { xs: 0, sm: 2 },
              mt: { xs: 0.5, sm: 0 },
            }}
          >
            {action}
          </Box>
        )}
      </Box>

      {children}
    </Box>
  );
}
