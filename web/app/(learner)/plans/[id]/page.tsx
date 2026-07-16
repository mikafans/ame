"use client";

import { useState, useEffect, use } from "react";
import { useRouter } from "next/navigation";
import { api } from "@/api/client";
import { formatDuration, formatDate } from "@/utils/format";
import { Button } from "@/components/ui/button";

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
  const router = useRouter();
  const [plan, setPlan] = useState<StudyPlan | null>(null);
  const [loading, setLoading] = useState(true);
  const [notFound, setNotFound] = useState(false);
  useEffect(() => {
    (api as any)
      .GET("/v1/plans/{id}", { params: { path: { id } } })
      .then(({ data, error }: { data?: StudyPlan; error?: unknown }) => {
        if (error || !data) setNotFound(true);
        else setPlan(data);
      })
      .catch(() => setNotFound(true))
      .finally(() => setLoading(false));
  }, [id]);
  if (loading)
    return (
      <div className="flex items-center gap-2 p-8 text-sm text-muted-foreground">
        <span className="size-5 animate-spin rounded-full border-2 border-primary border-t-transparent" />
        Loading plan…
      </div>
    );
  if (notFound || !plan)
    return (
      <div className="p-8">
        <p className="mb-4 text-muted-foreground">Plan not found.</p>
        <Button variant="outline" onClick={() => router.push("/progress")}>
          ← Back to progress
        </Button>
      </div>
    );
  const totalHours = plan.weeks
    .flatMap((w) => w.items)
    .reduce((sum, item) => sum + item.hours_est, 0);
  return (
    <div className="max-w-[760px] px-4 pb-16 pt-10 sm:px-12">
      <div className="mb-7">
        <p className="mb-2 font-mono text-xs uppercase tracking-[0.13em] text-muted-foreground">
          Study plan · {plan.weeks.length} weeks · {formatDuration(totalHours)}{" "}
          est.
        </p>
        <h1 className="mb-1 text-2xl font-medium">{plan.goal}</h1>
        <p className="text-xs text-muted-foreground">
          Generated {formatDate(plan.generated_at)} · based on last{" "}
          {plan.lookback_days} days
        </p>
      </div>
      <div className="space-y-4">
        {plan.weeks.map((week) => (
          <section
            key={week.week_num}
            className="overflow-hidden rounded-lg border border-border"
          >
            <div className="flex items-center justify-between border-b border-border px-5 py-4">
              <div className="flex items-center gap-3">
                <span className="font-mono text-xs uppercase tracking-[0.12em] text-muted-foreground">
                  Week {week.week_num}
                </span>
                <span className="text-sm font-semibold">{week.focus}</span>
              </div>
              <span className="font-mono text-xs text-muted-foreground">
                {formatDuration(
                  week.items.reduce((s, i) => s + i.hours_est, 0),
                )}
              </span>
            </div>
            <div className="space-y-2 p-4">
              {week.items.map((item, idx) => (
                <div
                  key={idx}
                  className="flex items-center justify-between rounded-md bg-muted/50 p-2 pl-3"
                >
                  <div className="flex items-center gap-3">
                    <span className="rounded-full border border-primary/50 px-2 py-0.5 text-[9px] uppercase tracking-wider text-primary">
                      {item.kind}
                    </span>
                    <span className="font-mono text-xs text-muted-foreground">
                      {item.ref_id}
                    </span>
                  </div>
                  <span className="font-mono text-xs text-muted-foreground">
                    {formatDuration(item.hours_est)}
                  </span>
                </div>
              ))}
            </div>
          </section>
        ))}
      </div>
      <div className="mt-7">
        <Button variant="outline" onClick={() => router.push("/progress")}>
          ← Back to progress
        </Button>
      </div>
    </div>
  );
}
