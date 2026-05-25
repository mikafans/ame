"use client";

import { useState, useEffect, use } from "react";
import { useRouter } from "next/navigation";
import { makeClient } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";
import { Button, Card } from "@/components/ui";

interface PlanItem {
  kind: string;
  ref_id: string;
  hours_est: number;
}

interface PlanWeek {
  week_num: number;
  focus: string;
  items: PlanItem[];
}

interface StudyPlan {
  id: string;
  goal: string;
  lookback_days: number;
  generated_at: string;
  weeks: PlanWeek[];
}

export default function PlanPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  const { token } = useAuth();
  const router = useRouter();
  const [plan, setPlan] = useState<StudyPlan | null>(null);
  const [loading, setLoading] = useState(true);
  const [notFound, setNotFound] = useState(false);

  useEffect(() => {
    if (!token) return;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (makeClient(token) as any)
      .GET("/v1/plans/{id}", { params: { path: { id } } })
      .then(({ data, error }: { data?: StudyPlan; error?: unknown }) => {
        if (error || !data) {
          setNotFound(true);
        } else {
          setPlan(data);
        }
      })
      .catch(() => setNotFound(true))
      .finally(() => setLoading(false));
  }, [token, id]);

  if (loading) {
    return (
      <div
        style={{
          padding: "28px 36px",
          color: "var(--muted)",
          fontFamily: "var(--mono)",
          fontSize: 13,
        }}
      >
        Loading plan…
      </div>
    );
  }

  if (notFound || !plan) {
    return (
      <div style={{ padding: "28px 36px" }}>
        <div
          style={{
            color: "var(--muted)",
            fontFamily: "var(--mono)",
            fontSize: 13,
            marginBottom: 16,
          }}
        >
          Plan not found.
        </div>
        <Button
          variant="ghost"
          size="md"
          onClick={() => router.push("/progress")}
        >
          ← Back to progress
        </Button>
      </div>
    );
  }

  const totalHours = plan.weeks
    .flatMap((w) => w.items)
    .reduce((sum, item) => sum + item.hours_est, 0);

  return (
    <div style={{ padding: "28px 36px 56px", maxWidth: 760 }}>
      <div style={{ marginBottom: 28 }}>
        <div
          style={{
            fontFamily: "var(--mono)",
            fontSize: 10,
            letterSpacing: 1.3,
            textTransform: "uppercase",
            color: "var(--muted)",
            marginBottom: 6,
          }}
        >
          Study plan · {plan.weeks.length} weeks · {totalHours.toFixed(1)}h est.
        </div>
        <h1
          style={{
            margin: "0 0 8px",
            fontFamily: "var(--serif)",
            fontSize: 32,
            fontWeight: 500,
            letterSpacing: -0.5,
            color: "var(--text)",
          }}
        >
          {plan.goal}
        </h1>
        <div style={{ fontSize: 12, color: "var(--muted)" }}>
          Generated {new Date(plan.generated_at).toLocaleDateString()} · based
          on last {plan.lookback_days} days
        </div>
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
        {plan.weeks.map((week) => (
          <Card key={week.week_num} style={{ padding: 0 }}>
            <div
              style={{
                padding: "14px 20px",
                borderBottom: "1px solid var(--border)",
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
              }}
            >
              <div>
                <span
                  style={{
                    fontFamily: "var(--mono)",
                    fontSize: 10,
                    letterSpacing: 1.2,
                    textTransform: "uppercase",
                    color: "var(--muted)",
                    marginRight: 10,
                  }}
                >
                  Week {week.week_num}
                </span>
                <span
                  style={{
                    fontWeight: 600,
                    fontSize: 14,
                    color: "var(--text)",
                  }}
                >
                  {week.focus}
                </span>
              </div>
              <span
                style={{
                  fontFamily: "var(--mono)",
                  fontSize: 11,
                  color: "var(--muted)",
                }}
              >
                {week.items.reduce((s, i) => s + i.hours_est, 0).toFixed(1)}h
              </span>
            </div>
            <div
              style={{
                padding: "12px 20px",
                display: "flex",
                flexDirection: "column",
                gap: 8,
              }}
            >
              {week.items.map((item, idx) => (
                <div
                  key={idx}
                  style={{
                    display: "flex",
                    justifyContent: "space-between",
                    alignItems: "center",
                    padding: "8px 12px",
                    background: "var(--surface-2)",
                    borderRadius: 4,
                  }}
                >
                  <div
                    style={{ display: "flex", gap: 10, alignItems: "center" }}
                  >
                    <span
                      style={{
                        fontFamily: "var(--mono)",
                        fontSize: 9,
                        letterSpacing: 1,
                        textTransform: "uppercase",
                        color: "var(--accent)",
                        background: "var(--accent-dim)",
                        padding: "2px 6px",
                        borderRadius: 3,
                      }}
                    >
                      {item.kind}
                    </span>
                    <span
                      style={{
                        fontFamily: "var(--mono)",
                        fontSize: 11,
                        color: "var(--text-2)",
                      }}
                    >
                      {item.ref_id}
                    </span>
                  </div>
                  <span
                    style={{
                      fontFamily: "var(--mono)",
                      fontSize: 11,
                      color: "var(--muted)",
                    }}
                  >
                    {item.hours_est}h
                  </span>
                </div>
              ))}
            </div>
          </Card>
        ))}
      </div>

      <div style={{ marginTop: 28 }}>
        <Button
          variant="ghost"
          size="md"
          onClick={() => router.push("/progress")}
        >
          ← Back to progress
        </Button>
      </div>
    </div>
  );
}
