"use client";
import { useState, useEffect } from "react";
import { api } from "@/api/client";
import { errorMessage } from "@/api/errors";
import { Button } from "@/components/ui/button";
import type { components } from "@/api/generated/schema";
type QuotaSettings = components["schemas"]["QuotaSettings"];
type RateLimitSettings = components["schemas"]["RateLimitSettings"];
const input =
  "h-9 w-full rounded-md border border-input bg-background px-3 text-sm outline-none focus:ring-2 focus:ring-ring";
export default function SettingsPage() {
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [maintenanceMode, setMaintenanceMode] = useState(false);
  const [savingMaintenance, setSavingMaintenance] = useState(false);
  const [maintenanceError, setMaintenanceError] = useState<string | null>(null);
  const [confirm, setConfirm] = useState(false);
  const [confirmLoading, setConfirmLoading] = useState(false);
  const [ratelimit, setRatelimit] = useState<RateLimitSettings>({
    free: { burst: 0, rate: 0 },
    premium: { burst: 0, rate: 0 },
  });
  const [quota, setQuota] = useState<QuotaSettings>({
    agents: { free: 0, premium: 0 },
    assessments: { free: 0, premium: 0 },
    questions: { free: 0, premium: 0 },
  });
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  useEffect(() => {
    api
      .GET("/v1/admin/settings")
      .then(({ data, error }) => {
        if (error)
          setLoadError(
            "Failed to load settings: " + errorMessage(error, "Unknown error"),
          );
        else if (data) {
          setMaintenanceMode(data.maintenanceMode);
          setRatelimit(data.ratelimit);
          setQuota(data.quota);
        }
      })
      .catch(() =>
        setLoadError("An unexpected error occurred while fetching settings."),
      )
      .finally(() => setLoading(false));
  }, []);
  const updateMaintenance = async (value: boolean) => {
    setSavingMaintenance(true);
    setMaintenanceError(null);
    try {
      const { data, error } = await api.PUT("/v1/admin/settings", {
        body: { maintenanceMode: value },
      });
      if (error) {
        setMaintenanceError(
          "Failed to save: " + errorMessage(error, "Unknown error"),
        );
      } else if (data) setMaintenanceMode(data.maintenanceMode);
    } catch {
      setMaintenanceError("Network or unexpected server error.");
    } finally {
      setSavingMaintenance(false);
    }
  };
  const confirmMaintenance = async () => {
    setConfirmLoading(true);
    await updateMaintenance(true);
    setConfirm(false);
    setConfirmLoading(false);
  };
  const save = async () => {
    setSaving(true);
    setSaveError(null);
    setSaved(false);
    try {
      const { data, error } = await api.PUT("/v1/admin/settings", {
        body: { ratelimit, quota },
      });
      if (error)
        setSaveError("Failed to save: " + errorMessage(error, "Unknown error"));
      else if (data) {
        setRatelimit(data.ratelimit);
        setQuota(data.quota);
        setSaved(true);
        setTimeout(() => setSaved(false), 4000);
      }
    } catch {
      setSaveError("Network or unexpected server error.");
    } finally {
      setSaving(false);
    }
  };
  const setRate = (
    tier: "free" | "premium",
    field: "burst" | "rate",
    value: string,
  ) =>
    setRatelimit((p) => ({
      ...p,
      [tier]: { ...p[tier], [field]: Number(value) || 0 },
    }));
  const setQuotaValue = (
    category: "agents" | "assessments" | "questions",
    tier: "free" | "premium",
    value: string,
  ) =>
    setQuota((p) => ({
      ...p,
      [category]: { ...p[category], [tier]: Number(value) || 0 },
    }));
  if (loading)
    return (
      <main className="p-12 text-muted-foreground">Loading settings…</main>
    );
  return (
    <main className="max-w-5xl px-6 py-12 sm:px-12">
      <header className="mb-8">
        <h1 className="text-3xl font-bold">Platform Settings</h1>
        <p className="mt-2 text-muted-foreground">
          Configure maintenance mode, rate limits, and quota controls.
        </p>
      </header>
      {loadError && <Notice tone="error">{loadError}</Notice>}
      <section className="mb-6 rounded-lg border border-border p-6">
        <h2 className="mb-4 text-lg font-bold">Maintenance Mode</h2>
        {maintenanceError && <Notice tone="error">{maintenanceError}</Notice>}
        <label className="flex items-center gap-3 text-sm">
          <input
            type="checkbox"
            checked={maintenanceMode}
            disabled={savingMaintenance}
            onChange={(e) =>
              e.target.checked ? setConfirm(true) : updateMaintenance(false)
            }
            className="size-4 accent-primary"
          />
          {maintenanceMode
            ? "Maintenance mode is ON"
            : "Maintenance mode is OFF"}
          {savingMaintenance && (
            <span className="text-muted-foreground">Saving…</span>
          )}
        </label>
        <p className="mt-3 text-sm text-muted-foreground">
          When enabled, all non-admin users receive a 503 Service Unavailable
          response.
        </p>
      </section>
      <section className="mb-6 rounded-lg border border-border p-6">
        <h2 className="mb-5 text-lg font-bold">Rate Limits</h2>
        {saveError && <Notice tone="error">{saveError}</Notice>}
        {saved && (
          <Notice tone="success">
            Rate limits and quotas saved successfully.
          </Notice>
        )}
        <div className="grid gap-6 sm:grid-cols-2">
          {(["free", "premium"] as const).map((tier) => (
            <div key={tier}>
              <h3 className="mb-3 font-semibold capitalize">{tier} tier</h3>
              <Field
                label="Burst"
                value={ratelimit[tier].burst}
                onChange={(v) => setRate(tier, "burst", v)}
              />
              <Field
                label="Rate"
                value={ratelimit[tier].rate}
                onChange={(v) => setRate(tier, "rate", v)}
              />
            </div>
          ))}
        </div>
      </section>
      <section className="mb-6 rounded-lg border border-border p-6">
        <h2 className="mb-5 text-lg font-bold">Usage Quotas</h2>
        <div className="grid gap-6 sm:grid-cols-3">
          {(["agents", "assessments", "questions"] as const).map((category) => (
            <div key={category}>
              <h3 className="mb-3 font-semibold capitalize">{category}</h3>
              <Field
                label="Free tier"
                value={quota[category].free}
                onChange={(v) => setQuotaValue(category, "free", v)}
              />
              <Field
                label="Premium tier"
                value={quota[category].premium}
                onChange={(v) => setQuotaValue(category, "premium", v)}
              />
            </div>
          ))}
        </div>
        <Button className="mt-6" onClick={save} disabled={saving}>
          {saving ? "Saving…" : "Save"}
        </Button>
      </section>
      {confirm && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
          <div className="w-full max-w-md rounded-xl border border-border bg-background p-6 shadow-xl">
            <h2 className="text-lg font-bold">Enable Maintenance Mode</h2>
            <p className="mt-3 text-sm text-muted-foreground">
              This will return a 503 response to all non-admin users. Are you
              sure?
            </p>
            <div className="mt-6 flex justify-end gap-2">
              <Button
                variant="ghost"
                onClick={() => setConfirm(false)}
                disabled={confirmLoading}
              >
                Cancel
              </Button>
              <Button onClick={confirmMaintenance} disabled={confirmLoading}>
                {confirmLoading ? "Enabling…" : "Enable Maintenance Mode"}
              </Button>
            </div>
          </div>
        </div>
      )}
    </main>
  );
}
function Field({
  label,
  value,
  onChange,
}: {
  label: string;
  value: number;
  onChange: (value: string) => void;
}) {
  return (
    <label className="mb-3 block text-xs text-muted-foreground">
      {label}
      <input
        className={`${input} mt-1`}
        type="number"
        value={value}
        onChange={(e) => onChange(e.target.value)}
      />
    </label>
  );
}
function Notice({
  tone,
  children,
}: {
  tone: "error" | "success";
  children: React.ReactNode;
}) {
  return (
    <div
      className={`mb-4 rounded-md border px-3 py-2 text-sm ${tone === "error" ? "border-destructive/40 bg-destructive/10 text-destructive" : "border-emerald-500/40 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300"}`}
    >
      {children}
    </div>
  );
}
