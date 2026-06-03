"use client";

import React from "react";
import Box from "@mui/material/Box";
import Drawer from "@mui/material/Drawer";
import List from "@mui/material/List";
import ListItem from "@mui/material/ListItem";
import ListItemButton from "@mui/material/ListItemButton";
import ListItemIcon from "@mui/material/ListItemIcon";
import ListItemText from "@mui/material/ListItemText";
import Typography from "@mui/material/Typography";
import Avatar from "@mui/material/Avatar";
import Divider from "@mui/material/Divider";
import LayersOutlinedIcon from "@mui/icons-material/LayersOutlined";
import ExploreOutlinedIcon from "@mui/icons-material/ExploreOutlined";
import PlayArrowOutlinedIcon from "@mui/icons-material/PlayArrowOutlined";
import AssessmentOutlinedIcon from "@mui/icons-material/AssessmentOutlined";
import DashboardOutlinedIcon from "@mui/icons-material/DashboardOutlined";
import EditOutlinedIcon from "@mui/icons-material/EditOutlined";
import GradingOutlinedIcon from "@mui/icons-material/GradingOutlined";
import FormatListBulletedOutlinedIcon from "@mui/icons-material/FormatListBulletedOutlined";
import StyleOutlinedIcon from "@mui/icons-material/StyleOutlined";
import SmartToyOutlinedIcon from "@mui/icons-material/SmartToyOutlined";
import LogoutOutlinedIcon from "@mui/icons-material/LogoutOutlined";
import DarkModeOutlinedIcon from "@mui/icons-material/DarkModeOutlined";
import LightModeOutlinedIcon from "@mui/icons-material/LightModeOutlined";
import IconButton from "@mui/material/IconButton";
import Tooltip from "@mui/material/Tooltip";
import { Logo } from "@/components/Logo";
import { useAuth } from "@/hooks/useAuth";
import { useColorMode } from "@/components/ThemeRegistry";

const DRAWER_WIDTH = 232;

// Deterministic avatar color: same name always maps to the same hue, so the
// default (initials) avatar is distinguishable per-user instead of one flat
// theme color. Fixed saturation/lightness keep white initials legible.
function stringToColor(str: string): string {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    hash = str.charCodeAt(i) + ((hash << 5) - hash);
  }
  return `hsl(${Math.abs(hash) % 360}, 55%, 45%)`;
}

const ICON_MAP: Record<string, React.ReactElement> = {
  explore: <ExploreOutlinedIcon fontSize="small" />,
  stack: <LayersOutlinedIcon fontSize="small" />,
  take: <PlayArrowOutlinedIcon fontSize="small" />,
  results: <AssessmentOutlinedIcon fontSize="small" />,
  dashboard: <DashboardOutlinedIcon fontSize="small" />,
  author: <EditOutlinedIcon fontSize="small" />,
  grade: <GradingOutlinedIcon fontSize="small" />,
  questions: <FormatListBulletedOutlinedIcon fontSize="small" />,
  flashcards: <StyleOutlinedIcon fontSize="small" />,
  agent: <SmartToyOutlinedIcon fontSize="small" />,
};

interface SidebarProps {
  route: string;
  setRoute: (route: string) => void;
}

export function Sidebar({ route, setRoute }: SidebarProps) {
  const { user, logout: logoutContext } = useAuth();
  const { mode, toggle } = useColorMode();

  async function handleLogout() {
    await logoutContext();
  }

  const items = [
    { id: "explore", label: "Explore", icon: "explore", section: "Browse" },
    {
      id: "assessment",
      label: "Take assessment",
      icon: "take",
      section: "Learn",
    },
    {
      id: "flashcards",
      label: "Flashcards",
      icon: "flashcards",
      section: "Learn",
    },
    {
      id: "questions",
      label: "Question bank",
      icon: "questions",
      section: "Learn",
    },
    { id: "results", label: "Last results", icon: "results", section: "Learn" },
    { id: "dashboard", label: "Progress", icon: "dashboard", section: "Learn" },
    {
      id: "author",
      label: "Author studio",
      icon: "author",
      section: "Manage",
    },
    {
      id: "agent",
      label: "Agent API",
      icon: "agent",
      section: "Integrate",
    },
  ];

  const sections = ["Browse", "Learn", "Manage", "Integrate"];
  const initials =
    user?.displayName
      ?.split(" ")
      .map((n) => n[0])
      .join("")
      .toUpperCase()
      .slice(0, 2) ?? "?";
  const displayName = user?.displayName ?? "";

  return (
    <Drawer
      variant="permanent"
      sx={{
        width: DRAWER_WIDTH,
        flexShrink: 0,
        "& .MuiDrawer-paper": {
          width: DRAWER_WIDTH,
          boxSizing: "border-box",
          display: "flex",
          flexDirection: "column",
        },
      }}
    >
      <Box sx={{ p: 2.5, pb: 2, borderBottom: 1, borderColor: "divider" }}>
        <Logo />
        <Typography
          variant="caption"
          color="text.secondary"
          sx={{
            letterSpacing: 1.2,
            textTransform: "none",
            display: "block",
            mt: 0.5,
          }}
        >
          AME
        </Typography>
      </Box>

      <Box sx={{ flex: 1, overflowY: "auto", py: 1 }}>
        {sections.map((sec) => {
          const inSec = items.filter((i) => i.section === sec);
          if (!inSec.length) return null;
          return (
            <Box key={sec} sx={{ mb: 2 }}>
              <Typography
                variant="caption"
                sx={{
                  px: 1.5,
                  py: 0.75,
                  display: "block",
                  letterSpacing: 1.4,
                  textTransform: "uppercase",
                  color: "text.secondary",
                }}
              >
                {sec}
              </Typography>
              <List dense disablePadding>
                {inSec.map((it) => (
                  <ListItem key={it.id} disablePadding>
                    <ListItemButton
                      selected={route === it.id}
                      onClick={() => setRoute(it.id)}
                      sx={{ borderRadius: 1, mx: 0.5 }}
                    >
                      <ListItemIcon sx={{ minWidth: 32 }}>
                        {ICON_MAP[it.icon]}
                      </ListItemIcon>
                      <ListItemText
                        primary={it.label}
                        slotProps={{ primary: { sx: { fontSize: 13 } } }}
                      />
                    </ListItemButton>
                  </ListItem>
                ))}
              </List>
            </Box>
          );
        })}
      </Box>

      <Divider />
      <Box sx={{ p: 1.75, display: "flex", alignItems: "center", gap: 1.25 }}>
        <Avatar
          sx={{
            width: 32,
            height: 32,
            fontSize: 14,
            bgcolor: stringToColor(displayName || user?.email || "?"),
          }}
        >
          {initials}
        </Avatar>
        <Box sx={{ flex: 1, minWidth: 0 }}>
          <Tooltip title={displayName} disableHoverListener={!displayName}>
            <Typography variant="body2" noWrap sx={{ fontWeight: 500 }}>
              {displayName}
            </Typography>
          </Tooltip>
        </Box>
        <Tooltip title={mode === "dark" ? "Light mode" : "Dark mode"}>
          <IconButton size="small" onClick={toggle}>
            {mode === "dark" ? (
              <LightModeOutlinedIcon sx={{ fontSize: 16 }} />
            ) : (
              <DarkModeOutlinedIcon sx={{ fontSize: 16 }} />
            )}
          </IconButton>
        </Tooltip>
        <Tooltip title="Sign out">
          <IconButton size="small" onClick={handleLogout}>
            <LogoutOutlinedIcon sx={{ fontSize: 16 }} />
          </IconButton>
        </Tooltip>
      </Box>
    </Drawer>
  );
}
