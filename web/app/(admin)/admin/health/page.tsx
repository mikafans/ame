"use client";
import { useState, useEffect, useCallback } from "react";
import { api } from "@/api/client";
import { errorMessage } from "@/api/errors";
import { Button } from "@/components/ui/button";
import {
  Activity,
  AlertTriangle,
  CheckCircle2,
  Database,
  FileClock,
  List,
  Play,
  RefreshCw,
  Shield,
  Users,
} from "lucide-react";
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
      if (apiError)
        setError(
          "Failed to fetch system status: " +
            errorMessage(apiError, "Unknown error"),
        );
      else if (data) {
        setHealth(data as HealthData);
        setLastUpdated(new Date().toLocaleTimeString());
      }
    } catch {
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
  const status = (value: string) => {
    const ok = ["ok", "healthy"].includes(value.toLowerCase());
    return (
      <span
        className={`inline-flex items-center gap-2 text-sm font-semibold ${ok ? "text-emerald-600 dark:text-emerald-400" : value === "degraded" ? "text-amber-600" : "text-red-600"}`}
      >
        {ok ? <CheckCircle2 size={18} /> : <AlertTriangle size={18} />}
        {ok
          ? "ONLINE"
          : value === "degraded"
            ? "DEGRADED"
            : `OFFLINE (${value})`}
      </span>
    );
  };
  const metrics = [
    ["Total Users", health?.usersCount ?? 0, Users],
    ["Total Agents", health?.agentsCount ?? 0, Activity],
    ["Assessments", health?.assessmentsCount ?? 0, List],
    ["Sessions", health?.sessionsCount ?? 0, Play],
    ["Questions", health?.questionsCount ?? 0, Database],
  ] as const;
  return (
    <main className="px-6 py-12 sm:px-12">
      <header className="mb-10 flex flex-wrap items-start justify-between gap-4">
        <div>
          <h1 className="text-3xl font-bold">System Health</h1>
          <p className="mt-2 text-muted-foreground">
            Monitor infrastructure services, database tables, and rate-limiting
            metrics.
          </p>
          {lastUpdated && (
            <p className="mt-1 text-xs text-muted-foreground">
              Last checked: {lastUpdated}
            </p>
          )}
        </div>
        <Button variant="outline" onClick={fetchHealth} disabled={loading}>
          <RefreshCw size={16} className={loading ? "animate-spin" : ""} />
          Refresh Status
        </Button>
      </header>
      {error && (
        <div
          role="alert"
          className="mb-8 rounded-md border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive"
        >
          {error}
        </div>
      )}
      <h2 className="mb-4 text-lg font-bold">Core Infrastructure</h2>
      <div className="mb-10 grid gap-5 sm:grid-cols-2">
        <Service
          title="Database Service (PostgreSQL)"
          subtitle="Primary Database"
        >
          {loading ? "Checking…" : status(health?.database ?? "unknown")}
        </Service>
        <Service
          title="Limiter Cache Service (Valkey)"
          subtitle="Rate Limiting"
        >
          {loading ? "Checking…" : status(health?.valkey ?? "unknown")}
        </Service>
      </div>
      <h2 className="mb-4 text-lg font-bold">Database Storage Row Counts</h2>
      <div className="mb-10 grid gap-4 sm:grid-cols-2 lg:grid-cols-5">
        {metrics.map(([title, count, Icon]) => (
          <div key={title} className="rounded-lg border border-border p-5">
            <Icon size={24} className="mb-4 text-primary" />
            <p className="text-xs text-muted-foreground">{title}</p>
            <p className="mt-1 text-2xl font-bold">
              {loading ? "…" : count.toLocaleString()}
            </p>
          </div>
        ))}
      </div>
      <h2 className="mb-4 text-lg font-bold">Operational Metrics</h2>
      <div className="grid gap-5 sm:grid-cols-2">
        <Metric
          icon={FileClock}
          title="Audit Log Entries Count"
          value={health?.auditLogCount ?? 0}
        >
          Append-only log total size.
        </Metric>
        <Metric
          icon={Shield}
          title="Rate-Limit Quota Rejections"
          value={health?.quotaRejectionsTotal ?? 0}
          warning={(health?.quotaRejectionsTotal ?? 0) > 0}
        >
          Recent client quota requests rejected by Valkey.
        </Metric>
      </div>
    </main>
  );
}
function Service({
  title,
  subtitle,
  children,
}: {
  title: string;
  subtitle: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between rounded-lg border border-border p-6">
      <div>
        <p className="text-sm text-muted-foreground">{title}</p>
        <p className="mt-1 font-semibold">{subtitle}</p>
      </div>
      {children}
    </div>
  );
}
function Metric({
  icon: Icon,
  title,
  value,
  warning,
  children,
}: {
  icon: typeof Shield;
  title: string;
  value: number;
  warning?: boolean;
  children: React.ReactNode;
}) {
  return (
    <div
      className={`flex items-center gap-5 rounded-lg border p-6 ${warning ? "border-amber-500/50" : "border-border"}`}
    >
      <Icon size={30} className={warning ? "text-amber-500" : "text-primary"} />
      <div>
        <p className="text-sm text-muted-foreground">{title}</p>
        <p
          className={`mt-1 text-3xl font-bold ${warning ? "text-amber-600" : ""}`}
        >
          {value.toLocaleString()}
        </p>
        <p className="mt-1 text-xs text-muted-foreground">{children}</p>
      </div>
    </div>
  );
}
