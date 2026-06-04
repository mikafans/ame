"use client";

import React, { useState } from "react";
import Box from "@mui/material/Box";
import AppBar from "@mui/material/AppBar";
import Toolbar from "@mui/material/Toolbar";
import IconButton from "@mui/material/IconButton";
import MenuOutlinedIcon from "@mui/icons-material/MenuOutlined";
import { Sidebar } from "@/components/Sidebar";
import { Logo } from "@/components/Logo";

interface AppShellProps {
  route: string;
  setRoute: (route: string) => void;
  children: React.ReactNode;
}

export function AppShell({ route, setRoute, children }: AppShellProps) {
  const [mobileOpen, setMobileOpen] = useState(false);

  const handleRouteChange = (r: string) => {
    setMobileOpen(false);
    setRoute(r);
  };

  const handleMobileClose = () => {
    setMobileOpen(false);
  };

  return (
    <Box sx={{ display: "flex", minHeight: "100vh" }}>
      <Sidebar
        route={route}
        setRoute={handleRouteChange}
        mobileOpen={mobileOpen}
        onClose={handleMobileClose}
      />
      <Box component="main" sx={{ flex: 1, minWidth: 0, overflowY: "auto" }}>
        <AppBar
          position="sticky"
          color="default"
          elevation={0}
          sx={{
            display: { md: "none" },
            bgcolor: "background.default",
            borderBottom: 1,
            borderColor: "divider",
          }}
        >
          <Toolbar variant="dense">
            <IconButton
              edge="start"
              aria-label="Open navigation"
              onClick={() => setMobileOpen(true)}
              sx={{ mr: 2 }}
            >
              <MenuOutlinedIcon />
            </IconButton>
            <Logo size={20} />
          </Toolbar>
        </AppBar>
        {children}
      </Box>
    </Box>
  );
}
