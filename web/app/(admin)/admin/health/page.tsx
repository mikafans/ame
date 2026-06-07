"use client";

import React, { useState, useEffect, useCallback } from "react";
import Box from "@mui/material/Box";
import Container from "@mui/material/Container";
import Typography from "@mui/material/Typography";
import Paper from "@mui/material/Paper";
import Grid from "@mui/material/Grid";
import Button from "@mui/material/Button";
import Card from "@mui/material/Card";
import CardContent from "@mui/material/CardContent";
import RefreshIcon from "@mui/icons-material/Refresh";
import CheckCircleIcon from "@mui/icons-material/CheckCircle";
import WarningIcon from "@mui/icons-material/Warning";
import ErrorIcon from "@mui/icons-material/Error";
import PeopleOutlinedIcon from "@mui/icons-material/PeopleOutlined";
import AssignmentOutlinedIcon from "@mui/icons-material/AssignmentOutlined";
import PlayCircleOutlinedIcon from "@mui/icons-material/PlayCircleOutlined";
import FormatListBulletedOutlinedIcon from "@mui/icons-material/FormatListBulletedOutlined";
import SmartToyOutlinedIcon from "@mui/icons-material/SmartToyOutlined";
import ShieldOutlinedIcon from "@mui/icons-material/ShieldOutlined";
import HistoryOutlinedIcon from "@mui/icons-material/HistoryOutlined";
import Alert from "@mui/material/Alert";
import CircularProgress from "@mui/material/CircularProgress";
import { api } from "@/api/client";

interface HealthData {
  database: string;
  valkey: string;
  usersCount: number;
  agentsCount: number;
  assessmentsCount: number;
  sessionsCount: number;
  questionsCount: number;
  auditLogCount: number;
  quotaRejectionsTotal: number;
}

export default function SystemHealthPage() {
  const [health, setHealth] = useState<HealthData | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [lastUpdated, setLastUpdated] = useState<string | null>(null);

  const fetchHealth = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const { data, error: apiError } = await api.GET("/v1/admin/health");

      if (apiError) {
        setError(
          "Failed to fetch system status: " + (apiError as any)?.message,
        );
        return;
      }

      if (data) {
        setHealth(data as HealthData);
        setLastUpdated(new Date().toLocaleTimeString());
      }
    } catch (err) {
      console.error(err);
      setError(
        "An unexpected error occurred while communicating with the server.",
      );
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchHealth();
  }, [fetchHealth]);

  const getStatusComponent = (status: string) => {
    const isOk =
      status.toLowerCase() === "ok" || status.toLowerCase() === "healthy";
    const isDegraded = status.toLowerCase() === "degraded";

    if (isOk) {
      return (
        <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
          <CheckCircleIcon color="success" />
          <Typography variant="body2" fontWeight="600" color="success.main">
            ONLINE
          </Typography>
        </Box>
      );
    } else if (isDegraded) {
      return (
        <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
          <WarningIcon color="warning" />
          <Typography variant="body2" fontWeight="600" color="warning.main">
            DEGRADED
          </Typography>
        </Box>
      );
    } else {
      return (
        <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
          <ErrorIcon color="error" />
          <Typography variant="body2" fontWeight="600" color="error.main">
            OFFLINE ({status})
          </Typography>
        </Box>
      );
    }
  };

  const dbStatus = health?.database ?? "unknown";
  const valkeyStatus = health?.valkey ?? "unknown";

  return (
    <Container maxWidth="lg" sx={{ py: 6 }}>
      {/* Header */}
      <Box
        sx={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "flex-start",
          mb: 5,
          gap: 2,
        }}
      >
        <Box>
          <Typography
            variant="h4"
            component="h1"
            fontWeight="bold"
            gutterBottom
          >
            System Health
          </Typography>
          <Typography variant="body1" color="text.secondary">
            Monitor infrastructure services, database tables, and rate-limiting
            metrics.
          </Typography>
          {lastUpdated && (
            <Typography
              variant="caption"
              color="text.secondary"
              sx={{ display: "block", mt: 0.5 }}
            >
              Last checked: {lastUpdated}
            </Typography>
          )}
        </Box>
        <Button
          variant="outlined"
          startIcon={
            loading ? (
              <CircularProgress size={16} color="inherit" />
            ) : (
              <RefreshIcon />
            )
          }
          onClick={fetchHealth}
          disabled={loading}
          sx={{ textTransform: "none", borderRadius: 2 }}
        >
          Refresh Status
        </Button>
      </Box>

      {error && (
        <Box sx={{ mb: 4 }}>
          <Alert severity="error">{error}</Alert>
        </Box>
      )}

      {/* Services Status Cards */}
      <Typography variant="h6" fontWeight="bold" sx={{ mb: 2.5 }}>
        Core Infrastructure
      </Typography>
      <Grid container spacing={3.5} sx={{ mb: 5 }}>
        <Grid size={{ xs: 12, sm: 6 }}>
          <Card
            sx={{
              borderRadius: 3,
              border: "1px solid",
              borderColor: "divider",
              boxShadow: "0 4px 12px rgba(0,0,0,0.01)",
              bgcolor: (theme) =>
                theme.palette.mode === "dark"
                  ? "rgba(255,255,255,0.01)"
                  : "rgba(0,0,0,0.005)",
            }}
          >
            <CardContent
              sx={{
                p: 3.5,
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
              }}
            >
              <Box>
                <Typography
                  variant="subtitle2"
                  color="text.secondary"
                  gutterBottom
                >
                  Database Service (PostgreSQL)
                </Typography>
                <Typography variant="h6" fontWeight="bold" sx={{ mt: 1 }}>
                  Primary Database
                </Typography>
              </Box>
              <Box>
                {loading ? (
                  <CircularProgress size={24} />
                ) : (
                  getStatusComponent(dbStatus)
                )}
              </Box>
            </CardContent>
          </Card>
        </Grid>

        <Grid size={{ xs: 12, sm: 6 }}>
          <Card
            sx={{
              borderRadius: 3,
              border: "1px solid",
              borderColor: "divider",
              boxShadow: "0 4px 12px rgba(0,0,0,0.01)",
              bgcolor: (theme) =>
                theme.palette.mode === "dark"
                  ? "rgba(255,255,255,0.01)"
                  : "rgba(0,0,0,0.005)",
            }}
          >
            <CardContent
              sx={{
                p: 3.5,
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
              }}
            >
              <Box>
                <Typography
                  variant="subtitle2"
                  color="text.secondary"
                  gutterBottom
                >
                  Limiter Cache Service (Valkey)
                </Typography>
                <Typography variant="h6" fontWeight="bold" sx={{ mt: 1 }}>
                  Rate Limiting
                </Typography>
              </Box>
              <Box>
                {loading ? (
                  <CircularProgress size={24} />
                ) : (
                  getStatusComponent(valkeyStatus)
                )}
              </Box>
            </CardContent>
          </Card>
        </Grid>
      </Grid>

      {/* Database Table Metrics */}
      <Typography variant="h6" fontWeight="bold" sx={{ mb: 2.5 }}>
        Database Storage Row Counts
      </Typography>
      <Grid container spacing={3.5} sx={{ mb: 5 }}>
        {[
          {
            title: "Total Users",
            count: health?.usersCount ?? 0,
            icon: (
              <PeopleOutlinedIcon
                sx={{ color: "primary.main", fontSize: 28 }}
              />
            ),
          },
          {
            title: "Total Agents",
            count: health?.agentsCount ?? 0,
            icon: (
              <SmartToyOutlinedIcon
                sx={{ color: "secondary.main", fontSize: 28 }}
              />
            ),
          },
          {
            title: "Assessments",
            count: health?.assessmentsCount ?? 0,
            icon: (
              <AssignmentOutlinedIcon
                sx={{ color: "success.main", fontSize: 28 }}
              />
            ),
          },
          {
            title: "Active Sessions",
            count: health?.sessionsCount ?? 0,
            icon: (
              <PlayCircleOutlinedIcon
                sx={{ color: "warning.main", fontSize: 28 }}
              />
            ),
          },
          {
            title: "Questions Bank",
            count: health?.questionsCount ?? 0,
            icon: (
              <FormatListBulletedOutlinedIcon
                sx={{ color: "info.main", fontSize: 28 }}
              />
            ),
          },
        ].map((item) => (
          <Grid size={{ xs: 12, sm: 6, md: 2.4 }} key={item.title}>
            <Paper
              variant="outlined"
              sx={{
                p: 3,
                borderRadius: 3,
                display: "flex",
                alignItems: "center",
                gap: 2.5,
                boxShadow: "0 2px 8px rgba(0,0,0,0.01)",
              }}
            >
              <Box
                sx={{
                  p: 1.25,
                  borderRadius: 2,
                  bgcolor: (theme) =>
                    theme.palette.mode === "dark"
                      ? "rgba(255,255,255,0.03)"
                      : "rgba(0,0,0,0.015)",
                }}
              >
                {item.icon}
              </Box>
              <Box>
                <Typography
                  variant="caption"
                  color="text.secondary"
                  fontWeight={500}
                >
                  {item.title}
                </Typography>
                <Typography variant="h5" fontWeight="bold" sx={{ mt: 0.5 }}>
                  {loading ? (
                    <CircularProgress size={18} />
                  ) : (
                    item.count.toLocaleString()
                  )}
                </Typography>
              </Box>
            </Paper>
          </Grid>
        ))}
      </Grid>

      {/* Operational Metrics */}
      <Typography variant="h6" fontWeight="bold" sx={{ mb: 2.5 }}>
        Operational Metrics
      </Typography>
      <Grid container spacing={3.5}>
        <Grid size={{ xs: 12, sm: 6 }}>
          <Paper
            variant="outlined"
            sx={{
              p: 3.5,
              borderRadius: 3,
              display: "flex",
              alignItems: "center",
              gap: 3,
              height: "100%",
            }}
          >
            <Box
              sx={{
                p: 1.5,
                borderRadius: 2,
                bgcolor: "info.light",
                color: "info.contrastText",
              }}
            >
              <HistoryOutlinedIcon sx={{ fontSize: 32 }} />
            </Box>
            <Box>
              <Typography
                variant="body2"
                color="text.secondary"
                fontWeight={500}
              >
                Audit Log Entries Count
              </Typography>
              <Typography variant="h4" fontWeight="bold" sx={{ mt: 0.5 }}>
                {loading ? (
                  <CircularProgress size={24} />
                ) : (
                  (health?.auditLogCount.toLocaleString() ?? "0")
                )}
              </Typography>
              <Typography
                variant="caption"
                color="text.secondary"
                sx={{ display: "block", mt: 0.5 }}
              >
                Append-only log total size.
              </Typography>
            </Box>
          </Paper>
        </Grid>

        <Grid size={{ xs: 12, sm: 6 }}>
          <Paper
            variant="outlined"
            sx={{
              p: 3.5,
              borderRadius: 3,
              display: "flex",
              alignItems: "center",
              gap: 3,
              height: "100%",
              borderColor:
                (health?.quotaRejectionsTotal ?? 0) > 0
                  ? "warning.main"
                  : "divider",
            }}
          >
            <Box
              sx={{
                p: 1.5,
                borderRadius: 2,
                bgcolor:
                  (health?.quotaRejectionsTotal ?? 0) > 0
                    ? "warning.light"
                    : "success.light",
                color:
                  (health?.quotaRejectionsTotal ?? 0) > 0
                    ? "warning.contrastText"
                    : "success.contrastText",
              }}
            >
              <ShieldOutlinedIcon sx={{ fontSize: 32 }} />
            </Box>
            <Box>
              <Typography
                variant="body2"
                color="text.secondary"
                fontWeight={500}
              >
                Rate-Limit Quota Rejections
              </Typography>
              <Typography
                variant="h4"
                fontWeight="bold"
                color={
                  (health?.quotaRejectionsTotal ?? 0) > 0
                    ? "warning.main"
                    : "text.primary"
                }
                sx={{ mt: 0.5 }}
              >
                {loading ? (
                  <CircularProgress size={24} />
                ) : (
                  (health?.quotaRejectionsTotal.toLocaleString() ?? "0")
                )}
              </Typography>
              <Typography
                variant="caption"
                color="text.secondary"
                sx={{ display: "block", mt: 0.5 }}
              >
                Recent client quota requests rejected by Valkey.
              </Typography>
            </Box>
          </Paper>
        </Grid>
      </Grid>
    </Container>
  );
}
