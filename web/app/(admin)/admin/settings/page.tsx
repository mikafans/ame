"use client";

import React, { useState, useEffect } from "react";
import Box from "@mui/material/Box";
import Container from "@mui/material/Container";
import Typography from "@mui/material/Typography";
import Paper from "@mui/material/Paper";
import TextField from "@mui/material/TextField";
import Switch from "@mui/material/Switch";
import FormControlLabel from "@mui/material/FormControlLabel";
import Button from "@mui/material/Button";
import Alert from "@mui/material/Alert";
import CircularProgress from "@mui/material/CircularProgress";
import Dialog from "@mui/material/Dialog";
import DialogTitle from "@mui/material/DialogTitle";
import DialogContent from "@mui/material/DialogContent";
import DialogContentText from "@mui/material/DialogContentText";
import DialogActions from "@mui/material/DialogActions";
import { api } from "@/api/client";
import { errorMessage } from "@/api/errors";
import { components } from "@/api/generated/schema";

type EffectiveSettings = components["schemas"]["EffectiveSettings"];
type QuotaSettings = components["schemas"]["QuotaSettings"];
type RateLimitSettings = components["schemas"]["RateLimitSettings"];

export default function SettingsPage() {
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);

  // Maintenance mode
  const [maintenanceMode, setMaintenanceMode] = useState(false);
  const [maintenanceSaving, setMaintenanceSaving] = useState(false);
  const [maintenanceError, setMaintenanceError] = useState<string | null>(null);

  // Dialog for turning on maintenance mode
  const [maintenanceDialogOpen, setMaintenanceDialogOpen] = useState(false);
  const [maintenanceDialogLoading, setMaintenanceDialogLoading] =
    useState(false);

  // Rate limits
  const [ratelimit, setRatelimit] = useState<RateLimitSettings>({
    free: { burst: 0, rate: 0 },
    premium: { burst: 0, rate: 0 },
  });

  // Quotas
  const [quota, setQuota] = useState<QuotaSettings>({
    agents: { free: 0, premium: 0 },
    assessments: { free: 0, premium: 0 },
    questions: { free: 0, premium: 0 },
  });

  const [rateLimitQuotaSaving, setRateLimitQuotaSaving] = useState(false);
  const [rateLimitQuotaError, setRateLimitQuotaError] = useState<string | null>(
    null,
  );
  const [rateLimitQuotaSuccess, setRateLimitQuotaSuccess] = useState(false);

  // Fetch settings on mount
  useEffect(() => {
    const fetchSettings = async () => {
      setLoading(true);
      setLoadError(null);
      try {
        const { data, error } = await api.GET("/v1/admin/settings");
        if (error) {
          setLoadError(
            "Failed to load settings: " + errorMessage(error, "Unknown error"),
          );
        } else if (data) {
          setMaintenanceMode(data.maintenanceMode);
          setRatelimit(data.ratelimit);
          setQuota(data.quota);
        }
      } catch (err) {
        console.error(err);
        setLoadError("An unexpected error occurred while fetching settings.");
      } finally {
        setLoading(false);
      }
    };
    fetchSettings();
  }, []);

  // Handle maintenance mode toggle
  const handleMaintenanceModeChange = (
    _: React.ChangeEvent<HTMLInputElement>,
    checked: boolean,
  ) => {
    if (checked) {
      // Turning ON — show dialog
      setMaintenanceDialogOpen(true);
    } else {
      // Turning OFF — save immediately
      handleSaveMaintenanceMode(false);
    }
  };

  // Confirm turning on maintenance mode via dialog
  const handleConfirmMaintenanceMode = async () => {
    setMaintenanceDialogLoading(true);
    setMaintenanceError(null);
    try {
      const { data, error } = await api.PUT("/v1/admin/settings", {
        body: { maintenanceMode: true },
      });
      if (error) {
        setMaintenanceError(
          "Failed to save: " + errorMessage(error, "Unknown error"),
        );
      } else if (data) {
        setMaintenanceMode(data.maintenanceMode);
        setMaintenanceDialogOpen(false);
      }
    } catch (err) {
      console.error(err);
      setMaintenanceError("Network or unexpected server error.");
    } finally {
      setMaintenanceDialogLoading(false);
    }
  };

  // Save maintenance mode OFF
  const handleSaveMaintenanceMode = async (value: boolean) => {
    setMaintenanceSaving(true);
    setMaintenanceError(null);
    try {
      const { data, error } = await api.PUT("/v1/admin/settings", {
        body: { maintenanceMode: value },
      });
      if (error) {
        setMaintenanceError(
          "Failed to save: " + errorMessage(error, "Unknown error"),
        );
        setMaintenanceMode(!value);
      } else if (data) {
        setMaintenanceMode(data.maintenanceMode);
      }
    } catch (err) {
      console.error(err);
      setMaintenanceError("Network or unexpected server error.");
      setMaintenanceMode(!value);
    } finally {
      setMaintenanceSaving(false);
    }
  };

  // Handle rate limit/quota save
  const handleSaveRateLimitQuota = async () => {
    setRateLimitQuotaSaving(true);
    setRateLimitQuotaError(null);
    setRateLimitQuotaSuccess(false);
    try {
      const { data, error } = await api.PUT("/v1/admin/settings", {
        body: { ratelimit, quota },
      });
      if (error) {
        setRateLimitQuotaError(
          "Failed to save: " + errorMessage(error, "Unknown error"),
        );
      } else if (data) {
        setRatelimit(data.ratelimit);
        setQuota(data.quota);
        setRateLimitQuotaSuccess(true);
        setTimeout(() => setRateLimitQuotaSuccess(false), 4000);
      }
    } catch (err) {
      console.error(err);
      setRateLimitQuotaError("Network or unexpected server error.");
    } finally {
      setRateLimitQuotaSaving(false);
    }
  };

  // Update rate limit field
  const handleRateLimitChange = (
    tier: "free" | "premium",
    field: "burst" | "rate",
    value: string,
  ) => {
    setRatelimit((prev) => ({
      ...prev,
      [tier]: {
        ...prev[tier],
        [field]: parseInt(value, 10) || 0,
      },
    }));
  };

  // Update quota field
  const handleQuotaChange = (
    category: "agents" | "assessments" | "questions",
    tier: "free" | "premium",
    value: string,
  ) => {
    setQuota((prev) => ({
      ...prev,
      [category]: {
        ...prev[category],
        [tier]: parseInt(value, 10) || 0,
      },
    }));
  };

  if (loading) {
    return (
      <Container maxWidth={false} sx={{ py: 6, px: { xs: 3, sm: 5 } }}>
        <Box sx={{ display: "flex", alignItems: "center", gap: 2 }}>
          <CircularProgress size={32} />
          <Typography color="text.secondary">Loading settings...</Typography>
        </Box>
      </Container>
    );
  }

  return (
    <Container maxWidth={false} sx={{ py: 6, px: { xs: 3, sm: 5 } }}>
      {/* Title */}
      <Box sx={{ mb: 4 }}>
        <Typography variant="h4" component="h1" fontWeight="bold" gutterBottom>
          Platform Settings
        </Typography>
        <Typography variant="body1" color="text.secondary">
          Configure platform-wide settings, including maintenance mode, rate
          limits, and quota controls.
        </Typography>
      </Box>

      {loadError && (
        <Alert severity="error" sx={{ mb: 3 }}>
          {loadError}
        </Alert>
      )}

      {/* Maintenance Mode Section */}
      <Paper
        sx={{
          borderRadius: 3,
          p: 3,
          mb: 3,
          border: "1px solid",
          borderColor: "divider",
          boxShadow: "0 4px 20px rgba(0,0,0,0.02)",
        }}
      >
        <Typography
          variant="h6"
          fontWeight="bold"
          sx={{ mb: 2, display: "flex", alignItems: "center", gap: 1 }}
        >
          Maintenance Mode
        </Typography>

        {maintenanceError && (
          <Alert severity="error" sx={{ mb: 2 }}>
            {maintenanceError}
          </Alert>
        )}

        <Box sx={{ display: "flex", alignItems: "center", gap: 2 }}>
          <FormControlLabel
            control={
              <Switch
                checked={maintenanceMode}
                onChange={handleMaintenanceModeChange}
                disabled={maintenanceSaving}
              />
            }
            label={
              maintenanceMode
                ? "Maintenance mode is ON"
                : "Maintenance mode is OFF"
            }
          />
          {maintenanceSaving && <CircularProgress size={20} />}
        </Box>

        <Typography
          variant="body2"
          color="text.secondary"
          sx={{ mt: 1.5, fontSize: 13 }}
        >
          When enabled, all non-admin users will receive a 503 Service
          Unavailable response.
        </Typography>
      </Paper>

      {/* Rate Limits Section */}
      <Paper
        sx={{
          borderRadius: 3,
          p: 3,
          mb: 3,
          border: "1px solid",
          borderColor: "divider",
          boxShadow: "0 4px 20px rgba(0,0,0,0.02)",
        }}
      >
        <Typography variant="h6" fontWeight="bold" sx={{ mb: 3 }}>
          Rate Limits
        </Typography>

        {rateLimitQuotaError && (
          <Alert severity="error" sx={{ mb: 2 }}>
            {rateLimitQuotaError}
          </Alert>
        )}

        {rateLimitQuotaSuccess && (
          <Alert severity="success" sx={{ mb: 2 }}>
            Rate limits and quotas saved successfully.
          </Alert>
        )}

        <Box
          sx={{
            display: "grid",
            gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr" },
            gap: 2.5,
            mb: 3,
          }}
        >
          <Box>
            <Typography variant="subtitle2" fontWeight="600" sx={{ mb: 2 }}>
              Free Tier
            </Typography>
            <TextField
              label="Burst"
              type="number"
              value={ratelimit.free.burst}
              onChange={(e) =>
                handleRateLimitChange("free", "burst", e.target.value)
              }
              fullWidth
              size="small"
              sx={{ mb: 1.5 }}
            />
            <TextField
              label="Rate"
              type="number"
              value={ratelimit.free.rate}
              onChange={(e) =>
                handleRateLimitChange("free", "rate", e.target.value)
              }
              fullWidth
              size="small"
            />
          </Box>

          <Box>
            <Typography variant="subtitle2" fontWeight="600" sx={{ mb: 2 }}>
              Premium Tier
            </Typography>
            <TextField
              label="Burst"
              type="number"
              value={ratelimit.premium.burst}
              onChange={(e) =>
                handleRateLimitChange("premium", "burst", e.target.value)
              }
              fullWidth
              size="small"
              sx={{ mb: 1.5 }}
            />
            <TextField
              label="Rate"
              type="number"
              value={ratelimit.premium.rate}
              onChange={(e) =>
                handleRateLimitChange("premium", "rate", e.target.value)
              }
              fullWidth
              size="small"
            />
          </Box>
        </Box>
      </Paper>

      {/* Quotas Section */}
      <Paper
        sx={{
          borderRadius: 3,
          p: 3,
          mb: 3,
          border: "1px solid",
          borderColor: "divider",
          boxShadow: "0 4px 20px rgba(0,0,0,0.02)",
        }}
      >
        <Typography variant="h6" fontWeight="bold" sx={{ mb: 3 }}>
          Usage Quotas
        </Typography>

        <Box
          sx={{
            display: "grid",
            gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr 1fr" },
            gap: 2.5,
            mb: 3,
          }}
        >
          {/* Agents */}
          <Box>
            <Typography variant="subtitle2" fontWeight="600" sx={{ mb: 2 }}>
              Agents
            </Typography>
            <TextField
              label="Free Tier"
              type="number"
              value={quota.agents.free}
              onChange={(e) =>
                handleQuotaChange("agents", "free", e.target.value)
              }
              fullWidth
              size="small"
              sx={{ mb: 1.5 }}
            />
            <TextField
              label="Premium Tier"
              type="number"
              value={quota.agents.premium}
              onChange={(e) =>
                handleQuotaChange("agents", "premium", e.target.value)
              }
              fullWidth
              size="small"
            />
          </Box>

          {/* Assessments */}
          <Box>
            <Typography variant="subtitle2" fontWeight="600" sx={{ mb: 2 }}>
              Assessments
            </Typography>
            <TextField
              label="Free Tier"
              type="number"
              value={quota.assessments.free}
              onChange={(e) =>
                handleQuotaChange("assessments", "free", e.target.value)
              }
              fullWidth
              size="small"
              sx={{ mb: 1.5 }}
            />
            <TextField
              label="Premium Tier"
              type="number"
              value={quota.assessments.premium}
              onChange={(e) =>
                handleQuotaChange("assessments", "premium", e.target.value)
              }
              fullWidth
              size="small"
            />
          </Box>

          {/* Questions */}
          <Box>
            <Typography variant="subtitle2" fontWeight="600" sx={{ mb: 2 }}>
              Questions
            </Typography>
            <TextField
              label="Free Tier"
              type="number"
              value={quota.questions.free}
              onChange={(e) =>
                handleQuotaChange("questions", "free", e.target.value)
              }
              fullWidth
              size="small"
              sx={{ mb: 1.5 }}
            />
            <TextField
              label="Premium Tier"
              type="number"
              value={quota.questions.premium}
              onChange={(e) =>
                handleQuotaChange("questions", "premium", e.target.value)
              }
              fullWidth
              size="small"
            />
          </Box>
        </Box>

        <Box sx={{ display: "flex", gap: 1.5 }}>
          <Button
            variant="contained"
            onClick={handleSaveRateLimitQuota}
            disabled={rateLimitQuotaSaving}
            sx={{ borderRadius: 2 }}
          >
            {rateLimitQuotaSaving ? (
              <>
                <CircularProgress size={16} sx={{ mr: 1 }} />
                Saving...
              </>
            ) : (
              "Save"
            )}
          </Button>
        </Box>
      </Paper>

      {/* Dialog: Confirm Maintenance Mode ON */}
      <Dialog
        open={maintenanceDialogOpen}
        onClose={() => {
          if (!maintenanceDialogLoading) {
            setMaintenanceDialogOpen(false);
          }
        }}
        fullWidth
        maxWidth="xs"
      >
        <DialogTitle fontWeight="bold">Enable Maintenance Mode</DialogTitle>
        <DialogContent dividers>
          <DialogContentText sx={{ mb: 2 }}>
            Enabling maintenance mode will return a 503 Service Unavailable
            response to all non-admin users. Are you sure?
          </DialogContentText>
        </DialogContent>
        <DialogActions sx={{ p: 2.5 }}>
          <Button
            onClick={() => setMaintenanceDialogOpen(false)}
            color="inherit"
            disabled={maintenanceDialogLoading}
          >
            Cancel
          </Button>
          <Button
            onClick={handleConfirmMaintenanceMode}
            variant="contained"
            color="warning"
            disabled={maintenanceDialogLoading}
          >
            {maintenanceDialogLoading ? (
              <CircularProgress size={20} />
            ) : (
              "Enable Maintenance Mode"
            )}
          </Button>
        </DialogActions>
      </Dialog>
    </Container>
  );
}
