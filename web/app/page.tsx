"use client";

import React, { useEffect, useState } from "react";
import Box from "@mui/material/Box";
import Container from "@mui/material/Container";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import Stack from "@mui/material/Stack";
import Card from "@mui/material/Card";
import Chip from "@mui/material/Chip";
import Grid from "@mui/material/Grid";
import Paper from "@mui/material/Paper";
import Divider from "@mui/material/Divider";
import IconButton from "@mui/material/IconButton";
import Tooltip from "@mui/material/Tooltip";
import ArrowForwardIcon from "@mui/icons-material/ArrowForward";
import SmartToyOutlinedIcon from "@mui/icons-material/SmartToyOutlined";
import LinkIcon from "@mui/icons-material/Link";
import ContentCopyIcon from "@mui/icons-material/ContentCopy";
import DarkModeOutlinedIcon from "@mui/icons-material/DarkModeOutlined";
import LightModeOutlinedIcon from "@mui/icons-material/LightModeOutlined";
import { useAuth } from "@/hooks/useAuth";
import { useColorMode } from "@/components/ThemeRegistry";

export default function LandingPage() {
  const [copied, setCopied] = useState(false);
  const { user } = useAuth();
  const { mode, toggle } = useColorMode();
  const isDark = mode === "dark";

  // The landing stays public (it's also the agent/discovery surface), but the
  // human CTA adapts: signed-in visitors get a one-click way back into the app,
  // signed-out visitors are sent to login. While useAuth is still resolving,
  // `user` is null so we default to the logged-out label.
  const signedIn = !!user;
  const ctaLabel = signedIn ? "Open Library" : "Use Now";
  const ctaHref = signedIn ? "/library" : "/login";

  // Resolve the origin only after mount. Computing it during render would
  // diverge between server (no window) and client and break hydration, so we
  // start empty (URLs render relative) and fill in the absolute origin once
  // the client has mounted.
  const [baseUrl, setBaseUrl] = useState("");
  useEffect(() => {
    setBaseUrl(window.location.origin);
  }, []);

  const starterPrompt = `You are an AI assistant helping me with my study on AME. AME has a first-class agent surface. To learn how to use it, please fetch and read the platform capabilities at:
${baseUrl}/llms.txt

The machine-readable tool schemas are available at:
${baseUrl}/v1/agents/skill.json

Authenticate all your requests using your Agent API Bearer Key.

Your first task is to read my learning stats at /v1/me/stats, identify my weakest topics, and create a targeted practice assessment to help me master them!`;

  const handleCopyPrompt = () => {
    navigator.clipboard.writeText(starterPrompt).catch(() => {});
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  // Mode-dependent palette. The page hardcodes its own colors (rather than
  // leaning on the MUI theme) so the hero gradients stay intentional, but each
  // value has a light + dark variant so text never goes invisible-on-invisible.
  const c = isDark
    ? {
        pageBg: "#0A0915",
        pageGradient: `radial-gradient(circle at 10% 20%, rgba(98, 0, 234, 0.15) 0%, transparent 40%),
                       radial-gradient(circle at 90% 80%, rgba(0, 229, 255, 0.1) 0%, transparent 40%)`,
        textPrimary: "#F8F9FA",
        textSecondary: "#90A4AE",
        headingGradient: "linear-gradient(135deg, #FFFFFF 0%, #90A4AE 100%)",
        logoGradient: "linear-gradient(90deg, #FFFFFF 0%, #B0BEC5 100%)",
        agentChipColor: "#B388FF",
        cardBg: "rgba(18, 16, 35, 0.65)",
        cardBorder: "1px solid rgba(255, 255, 255, 0.08)",
        cardShadow: "0 20px 40px rgba(0,0,0,0.4)",
        rowBg: "rgba(255, 255, 255, 0.03)",
        rowBorder: "1px solid rgba(255, 255, 255, 0.05)",
        rowHoverBg: "rgba(255, 255, 255, 0.06)",
        outlineBorder: "rgba(255, 255, 255, 0.15)",
        outlineColor: "#FFF",
        ghostBorder: "rgba(255, 255, 255, 0.1)",
        ghostColor: "#B0BEC5",
        promptBg: "rgba(0, 0, 0, 0.25)",
        divider: "rgba(255, 255, 255, 0.08)",
      }
    : {
        pageBg: "#F5F7FB",
        pageGradient: `radial-gradient(circle at 10% 20%, rgba(98, 0, 234, 0.08) 0%, transparent 40%),
                       radial-gradient(circle at 90% 80%, rgba(0, 229, 255, 0.08) 0%, transparent 40%)`,
        textPrimary: "#1A1A2E",
        textSecondary: "#5A6473",
        headingGradient: "linear-gradient(135deg, #1A1A2E 0%, #4A5568 100%)",
        logoGradient: "linear-gradient(90deg, #1A1A2E 0%, #4A5568 100%)",
        agentChipColor: "#6200EA",
        cardBg: "rgba(255, 255, 255, 0.85)",
        cardBorder: "1px solid rgba(0, 0, 0, 0.08)",
        cardShadow: "0 20px 40px rgba(98, 0, 234, 0.08)",
        rowBg: "rgba(98, 0, 234, 0.04)",
        rowBorder: "1px solid rgba(0, 0, 0, 0.06)",
        rowHoverBg: "rgba(98, 0, 234, 0.08)",
        outlineBorder: "rgba(0, 0, 0, 0.15)",
        outlineColor: "#1A1A2E",
        ghostBorder: "rgba(0, 0, 0, 0.15)",
        ghostColor: "#4A5568",
        promptBg: "rgba(0, 0, 0, 0.04)",
        divider: "rgba(0, 0, 0, 0.08)",
      };

  return (
    <Box
      sx={{
        minHeight: "100vh",
        bgcolor: c.pageBg,
        backgroundImage: c.pageGradient,
        color: c.textPrimary,
        display: "flex",
        flexDirection: "column",
        justifyContent: "center",
        fontFamily: "'Outfit', 'Inter', sans-serif",
        py: 8,
        overflow: "hidden",
      }}
    >
      <Container maxWidth="lg">
        {/* Top Header */}
        <Box
          sx={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            mb: 8,
          }}
        >
          <Box sx={{ display: "flex", alignItems: "center", gap: 1.5 }}>
            <Box
              sx={{
                width: 36,
                height: 36,
                borderRadius: "8px",
                background: "linear-gradient(135deg, #6200EA 0%, #00E5FF 100%)",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                boxShadow: "0 4px 14px rgba(98, 0, 234, 0.4)",
              }}
            >
              <SmartToyOutlinedIcon sx={{ color: "#FFF", fontSize: 20 }} />
            </Box>
            <Typography
              variant="h5"
              sx={{
                fontWeight: 700,
                letterSpacing: -0.5,
                background: c.logoGradient,
                WebkitBackgroundClip: "text",
                WebkitTextFillColor: "transparent",
              }}
            >
              ame
            </Typography>
          </Box>

          <Stack direction="row" alignItems="center" spacing={1.5}>
            <Tooltip title={isDark ? "Light mode" : "Dark mode"}>
              <IconButton
                onClick={toggle}
                sx={{ color: c.textSecondary }}
                size="small"
              >
                {isDark ? (
                  <LightModeOutlinedIcon fontSize="small" />
                ) : (
                  <DarkModeOutlinedIcon fontSize="small" />
                )}
              </IconButton>
            </Tooltip>
            <Button
              variant="outlined"
              onClick={() => (window.location.href = ctaHref)}
              sx={{
                borderColor: c.outlineBorder,
                color: c.outlineColor,
                textTransform: "none",
                borderRadius: "8px",
                px: 3,
                py: 0.75,
                fontSize: 14,
                fontWeight: 500,
                transition: "all 0.3s",
                "&:hover": {
                  borderColor: "#6200EA",
                  bgcolor: "rgba(98, 0, 234, 0.08)",
                  boxShadow: "0 0 15px rgba(98, 0, 234, 0.3)",
                },
              }}
            >
              {ctaLabel}
            </Button>
          </Stack>
        </Box>

        {/* Hero Section */}
        <Grid container spacing={6} alignItems="center">
          <Grid size={{ xs: 12, md: 6 }}>
            <Box sx={{ pr: { md: 4 } }}>
              <Chip
                label="First-Class Agent Support"
                sx={{
                  background: "rgba(98, 0, 234, 0.12)",
                  border: "1px solid rgba(98, 0, 234, 0.3)",
                  color: c.agentChipColor,
                  fontWeight: 600,
                  fontSize: 12,
                  mb: 3,
                  py: 1.5,
                }}
              />
              <Typography
                variant="h2"
                sx={{
                  fontWeight: 800,
                  letterSpacing: -1.5,
                  lineHeight: 1.15,
                  mb: 2.5,
                  background: c.headingGradient,
                  WebkitBackgroundClip: "text",
                  WebkitTextFillColor: "transparent",
                }}
              >
                Adaptive Study Powered by AI Agents.
              </Typography>
              <Typography
                variant="body1"
                sx={{
                  color: c.textSecondary,
                  fontSize: 17,
                  lineHeight: 1.6,
                  mb: 4.5,
                  maxWidth: 500,
                }}
              >
                AME is the modern assessment platform built equally for humans
                and AI assistants. Author question banks, analyze learner
                performance, and generate customized study plans — all over a
                unified programmatic surface.
              </Typography>

              <Stack direction={{ xs: "column", sm: "row" }} spacing={2.5}>
                <Button
                  variant="contained"
                  onClick={() => (window.location.href = ctaHref)}
                  endIcon={<ArrowForwardIcon />}
                  sx={{
                    background:
                      "linear-gradient(135deg, #6200EA 0%, #651FFF 100%)",
                    color: "#FFF",
                    textTransform: "none",
                    borderRadius: "10px",
                    px: 4,
                    py: 1.5,
                    fontSize: 16,
                    fontWeight: 600,
                    boxShadow: "0 6px 20px rgba(98, 0, 234, 0.3)",
                    transition: "all 0.3s",
                    "&:hover": {
                      background:
                        "linear-gradient(135deg, #7C4DFF 0%, #651FFF 100%)",
                      transform: "translateY(-2px)",
                      boxShadow: "0 8px 25px rgba(98, 0, 234, 0.5)",
                    },
                  }}
                >
                  {ctaLabel}
                </Button>
                <Button
                  variant="outlined"
                  onClick={() => window.open("/llms.txt", "_blank")}
                  sx={{
                    borderColor: c.ghostBorder,
                    color: c.ghostColor,
                    textTransform: "none",
                    borderRadius: "10px",
                    px: 3.5,
                    py: 1.5,
                    fontSize: 16,
                    fontWeight: 500,
                    transition: "all 0.3s",
                    "&:hover": {
                      borderColor: "#6200EA",
                      color: c.textPrimary,
                      bgcolor: "rgba(98, 0, 234, 0.06)",
                    },
                  }}
                >
                  Read llms.txt Specs
                </Button>
              </Stack>
            </Box>
          </Grid>

          {/* AI Discovery Card */}
          <Grid size={{ xs: 12, md: 6 }}>
            <Card
              variant="outlined"
              sx={{
                bgcolor: c.cardBg,
                backdropFilter: "blur(20px)",
                border: c.cardBorder,
                borderRadius: "16px",
                p: 4,
                boxShadow: c.cardShadow,
                position: "relative",
                "&::before": {
                  content: '""',
                  position: "absolute",
                  top: 0,
                  left: 0,
                  right: 0,
                  height: "3px",
                  background:
                    "linear-gradient(90deg, #6200EA 0%, #00E5FF 100%)",
                  borderTopLeftRadius: "16px",
                  borderTopRightRadius: "16px",
                },
              }}
            >
              <Box
                sx={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "flex-start",
                  mb: 3,
                }}
              >
                <Box>
                  <Typography
                    variant="h6"
                    sx={{
                      fontWeight: 700,
                      letterSpacing: -0.3,
                      mb: 0.5,
                      color: c.textPrimary,
                    }}
                  >
                    🤖 LLM Agent Discovery Surface
                  </Typography>
                  <Typography
                    variant="caption"
                    sx={{ color: c.textSecondary, display: "block" }}
                  >
                    Machine-readable configuration files served publicly
                  </Typography>
                </Box>
                <Chip
                  label="API v1"
                  color="primary"
                  size="small"
                  sx={{ fontWeight: 600 }}
                />
              </Box>

              <Stack spacing={2} sx={{ mb: 4 }}>
                {[
                  {
                    href: "/llms.txt",
                    label: "/llms.txt",
                    desc: "Architecture overview, scopes, and ELO dynamic programming playbooks.",
                  },
                  {
                    href: "/v1/agents/skill.json",
                    label: "/v1/agents/skill.json",
                    desc: "MCP-compatible tool definitions and schema parameters.",
                  },
                  {
                    href: "/v1/agents/openapi.json",
                    label: "/v1/agents/openapi.json",
                    desc: "Standard OpenAPI 3.1 schema for model client generators.",
                  },
                ].map((row) => (
                  <Box
                    key={row.href}
                    onClick={() => window.open(row.href, "_blank")}
                    sx={{
                      display: "flex",
                      justifyContent: "space-between",
                      alignItems: "center",
                      p: 2,
                      borderRadius: "10px",
                      bgcolor: c.rowBg,
                      border: c.rowBorder,
                      cursor: "pointer",
                      transition: "all 0.2s",
                      "&:hover": {
                        bgcolor: c.rowHoverBg,
                        borderColor: "#6200EA",
                        transform: "translateX(4px)",
                      },
                    }}
                  >
                    <Box>
                      <Typography
                        variant="subtitle2"
                        sx={{
                          fontFamily: "monospace",
                          fontWeight: 600,
                          color: c.textPrimary,
                        }}
                      >
                        {row.label}
                      </Typography>
                      <Typography
                        variant="caption"
                        sx={{ color: c.textSecondary }}
                      >
                        {row.desc}
                      </Typography>
                    </Box>
                    <LinkIcon sx={{ color: c.textSecondary, fontSize: 18 }} />
                  </Box>
                ))}
              </Stack>

              <Divider sx={{ borderColor: c.divider, mb: 3 }} />

              {/* Dynamic Starter Prompt Constructor */}
              <Box>
                <Box
                  sx={{
                    display: "flex",
                    justifyContent: "space-between",
                    alignItems: "center",
                    mb: 1.5,
                  }}
                >
                  <Typography
                    variant="subtitle2"
                    sx={{ fontWeight: 600, color: c.textPrimary }}
                  >
                    🚀 Boot Prompt for AI Assistants
                  </Typography>
                  <Button
                    size="small"
                    variant="text"
                    onClick={handleCopyPrompt}
                    startIcon={<ContentCopyIcon sx={{ fontSize: 14 }} />}
                    sx={{
                      color: copied ? "#00B8D4" : "#7C4DFF",
                      textTransform: "none",
                      fontSize: 12,
                    }}
                  >
                    {copied ? "Copied Prompt!" : "Copy Prompt"}
                  </Button>
                </Box>
                <Paper
                  variant="outlined"
                  sx={{
                    p: 2,
                    bgcolor: c.promptBg,
                    borderColor: c.divider,
                    borderRadius: "8px",
                    fontFamily: "monospace",
                    fontSize: 10.5,
                    lineHeight: 1.5,
                    maxHeight: 140,
                    overflowY: "auto",
                    color: c.textSecondary,
                    whiteSpace: "pre-wrap",
                  }}
                >
                  {starterPrompt}
                </Paper>
              </Box>
            </Card>
          </Grid>
        </Grid>
      </Container>
    </Box>
  );
}
