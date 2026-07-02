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
import LinkIcon from "@mui/icons-material/Link";
import ContentCopyIcon from "@mui/icons-material/ContentCopy";
import DarkModeOutlinedIcon from "@mui/icons-material/DarkModeOutlined";
import LightModeOutlinedIcon from "@mui/icons-material/LightModeOutlined";
import { useAuth } from "@/hooks/useAuth";
import { useColorMode } from "@/components/ThemeRegistry";
import { copyToClipboard } from "@/utils/clipboard";
import { Logo } from "@/components/Logo";
import { BRAND } from "@/lib/brand";

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
  const ctaLabel = signedIn ? "Open Explore" : "Use Now";
  const ctaHref = signedIn ? "/explore" : "/login";

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
${baseUrl}/skill.json

Authenticate all your requests using your Agent API Bearer Key.

Your first task is to read my learning stats at /v1/me/stats, identify my weakest topics, and create a targeted practice assessment to help me master them!`;

  const handleCopyPrompt = () => {
    copyToClipboard(starterPrompt).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  };

  // Mode-dependent palette. The page hardcodes its own colors (rather than
  // leaning on the MUI theme) so the hero gradients stay intentional, but each
  // value has a light + dark variant so text never goes invisible-on-invisible.
  const c = isDark
    ? {
        pageBg: BRAND.dark.background,
        pageGradient: `radial-gradient(circle at 10% 20%, rgba(98, 216, 205, 0.12) 0%, transparent 40%),
                       radial-gradient(circle at 90% 80%, rgba(255, 157, 192, 0.12) 0%, transparent 42%)`,
        textPrimary: BRAND.dark.text,
        textSecondary: "#90A4AE",
        headingGradient: "linear-gradient(135deg, #E2E8F0 0%, #62D8CD 100%)",
        agentChipColor: BRAND.dark.secondary,
        brandStart: BRAND.dark.primary,
        brandEnd: BRAND.dark.secondary,
        brandStartSoft: "rgba(98, 216, 205, 0.15)",
        brandEndSoft: "rgba(255, 157, 192, 0.16)",
        brandBorder: "rgba(98, 216, 205, 0.35)",
        brandShadow: "rgba(98, 216, 205, 0.28)",
        cardBg: "rgba(27, 42, 40, 0.78)",
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
        pageBg: BRAND.light.background,
        pageGradient: `radial-gradient(circle at 10% 20%, rgba(69, 196, 185, 0.14) 0%, transparent 42%),
                       radial-gradient(circle at 90% 80%, rgba(255, 143, 180, 0.16) 0%, transparent 42%)`,
        textPrimary: BRAND.light.text,
        textSecondary: "#5A6473",
        headingGradient: "linear-gradient(135deg, #0F172A 0%, #1F766F 100%)",
        agentChipColor: BRAND.light.secondaryDark,
        brandStart: BRAND.light.primary,
        brandEnd: BRAND.light.secondary,
        brandStartSoft: "rgba(69, 196, 185, 0.14)",
        brandEndSoft: "rgba(255, 143, 180, 0.16)",
        brandBorder: "rgba(47, 167, 158, 0.26)",
        brandShadow: "rgba(47, 167, 158, 0.16)",
        cardBg: "rgba(255, 255, 255, 0.85)",
        cardBorder: "1px solid rgba(0, 0, 0, 0.08)",
        cardShadow: "0 20px 40px rgba(47, 167, 158, 0.12)",
        rowBg: "rgba(69, 196, 185, 0.07)",
        rowBorder: "1px solid rgba(0, 0, 0, 0.06)",
        rowHoverBg: "rgba(69, 196, 185, 0.12)",
        outlineBorder: "rgba(0, 0, 0, 0.15)",
        outlineColor: "#0F172A",
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
          <Logo size={36} />

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
                  borderColor: c.brandStart,
                  bgcolor: c.brandStartSoft,
                  boxShadow: `0 0 15px ${c.brandShadow}`,
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
                  background: c.brandEndSoft,
                  border: `1px solid ${c.brandBorder}`,
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
                    background: `linear-gradient(135deg, ${c.brandStart} 0%, ${c.brandEnd} 100%)`,
                    color: "#0C1817",
                    textTransform: "none",
                    borderRadius: "10px",
                    px: 4,
                    py: 1.5,
                    fontSize: 16,
                    fontWeight: 600,
                    boxShadow: `0 6px 20px ${c.brandShadow}`,
                    transition: "all 0.3s",
                    "&:hover": {
                      background: `linear-gradient(135deg, ${c.brandEnd} 0%, ${c.brandStart} 100%)`,
                      transform: "translateY(-2px)",
                      boxShadow: `0 8px 25px ${c.brandShadow}`,
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
                      borderColor: c.brandStart,
                      color: c.textPrimary,
                      bgcolor: c.brandStartSoft,
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
                  background: `linear-gradient(90deg, ${c.brandStart} 0%, ${c.brandEnd} 100%)`,
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
                    href: "/skill.json",
                    label: "/skill.json",
                    desc: "MCP-compatible tool definitions and schema parameters.",
                  },
                  {
                    href: "/openapi.yaml",
                    label: "/openapi.yaml",
                    desc: "Standard OpenAPI 3.1 schema (YAML) for model client generators.",
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
                        borderColor: c.brandStart,
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
                      color: copied ? c.brandStart : c.brandEnd,
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
