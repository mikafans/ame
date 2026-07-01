"use client";

import React from "react";
import Box from "@mui/material/Box";
import Typography from "@mui/material/Typography";
import NotificationsNoneOutlinedIcon from "@mui/icons-material/NotificationsNoneOutlined";
import SearchOutlinedIcon from "@mui/icons-material/SearchOutlined";
import { useColorMode } from "@/components/ThemeRegistry";

interface TopbarProps {
  title: string;
  subtitle?: string;
  breadcrumb?: string;
  actions?: React.ReactNode;
}

export function Topbar({ title, subtitle, breadcrumb, actions }: TopbarProps) {
  const { mode } = useColorMode();
  const mascotSrc =
    mode === "dark" ? "/miku-icon-dark.svg" : "/miku-icon-light.svg";

  return (
    <Box
      component="header"
      sx={{
        px: 4.5,
        py: 2.5,
        borderBottom: 1,
        borderColor: "divider",
        bgcolor: "background.default",
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        position: "sticky",
        top: 0,
        zIndex: 5,
      }}
    >
      <Box sx={{ minWidth: 0 }}>
        <Box sx={{ display: "flex", alignItems: "center", gap: 1.25, mb: 1 }}>
          <Box
            component="img"
            src={mascotSrc}
            alt=""
            aria-hidden="true"
            sx={{
              width: 32,
              height: 32,
              display: "block",
              flex: "0 0 auto",
              borderRadius: 2,
            }}
          />
          <Typography
            component="span"
            sx={{
              fontSize: 24,
              fontWeight: 800,
              lineHeight: 1,
              letterSpacing: "-0.02em",
              background: "linear-gradient(120deg, #45C4B9, #FF8FB4)",
              backgroundClip: "text",
              WebkitBackgroundClip: "text",
              color: "transparent",
            }}
          >
            ame
          </Typography>
        </Box>
        {breadcrumb && (
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{
              letterSpacing: 1.3,
              textTransform: "uppercase",
              display: "block",
              mb: 0.75,
            }}
          >
            {breadcrumb}
          </Typography>
        )}
        <Typography variant="h5" sx={{ fontWeight: 500, letterSpacing: -0.3 }}>
          {title}
        </Typography>
        {subtitle && (
          <Typography variant="body2" color="text.secondary" sx={{ mt: 0.5 }}>
            {subtitle}
          </Typography>
        )}
      </Box>

      <Box sx={{ display: "flex", alignItems: "center", gap: 1.25 }}>
        {actions}
        <Box
          sx={{
            display: "flex",
            alignItems: "center",
            gap: 1.25,
            pl: 1.75,
            borderLeft: 1,
            borderColor: "divider",
          }}
        >
          <NotificationsNoneOutlinedIcon
            sx={{ fontSize: 18, color: "text.secondary" }}
          />
          <SearchOutlinedIcon sx={{ fontSize: 18, color: "text.secondary" }} />
        </Box>
      </Box>
    </Box>
  );
}
