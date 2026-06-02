"use client";

import { useState, useEffect, use } from "react";
import { useRouter } from "next/navigation";
import { api } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import Card from "@mui/material/Card";
import CardContent from "@mui/material/CardContent";
import Chip from "@mui/material/Chip";
import CircularProgress from "@mui/material/CircularProgress";
import { formatDuration, formatDate } from "@/utils/format";

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
  const { user } = useAuth();
  const router = useRouter();
  const [plan, setPlan] = useState<StudyPlan | null>(null);
  const [loading, setLoading] = useState(true);
  const [notFound, setNotFound] = useState(false);

  useEffect(() => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (api as any)
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
  }, [id]);

  if (loading) {
    return (
      <Box
        sx={{
          display: "flex",
          alignItems: "center",
          gap: 1,
          p: 4,
          color: "text.secondary",
        }}
      >
        <CircularProgress size={20} />
        <Typography variant="body2">Loading plan…</Typography>
      </Box>
    );
  }

  if (notFound || !plan) {
    return (
      <Box sx={{ p: 4 }}>
        <Typography color="text.secondary" sx={{ mb: 2 }}>
          Plan not found.
        </Typography>
        <Button variant="outlined" onClick={() => router.push("/progress")}>
          ← Back to progress
        </Button>
      </Box>
    );
  }

  const totalHours = plan.weeks
    .flatMap((w) => w.items)
    .reduce((sum, item) => sum + item.hours_est, 0);

  return (
    <Box sx={{ p: "28px 36px 56px", maxWidth: 760 }}>
      <Box sx={{ mb: 3.5 }}>
        <Typography
          variant="caption"
          color="text.secondary"
          sx={{
            fontFamily: "monospace",
            letterSpacing: 1.3,
            textTransform: "uppercase",
            display: "block",
            mb: 0.75,
          }}
        >
          Study plan · {plan.weeks.length} weeks · {formatDuration(totalHours)}{" "}
          est.
        </Typography>
        <Typography variant="h5" sx={{ fontWeight: 500, mb: 1 }}>
          {plan.goal}
        </Typography>
        <Typography variant="caption" color="text.secondary">
          Generated {formatDate(plan.generated_at)} · based on last{" "}
          {plan.lookback_days} days
        </Typography>
      </Box>

      <Stack spacing={2}>
        {plan.weeks.map((week) => (
          <Card key={week.week_num} variant="outlined">
            <Box
              sx={{
                px: 2.5,
                py: 1.75,
                borderBottom: 1,
                borderColor: "divider",
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
              }}
            >
              <Box sx={{ display: "flex", alignItems: "center", gap: 1.25 }}>
                <Typography
                  variant="caption"
                  color="text.secondary"
                  sx={{
                    fontFamily: "monospace",
                    letterSpacing: 1.2,
                    textTransform: "uppercase",
                  }}
                >
                  Week {week.week_num}
                </Typography>
                <Typography variant="body2" sx={{ fontWeight: 600 }}>
                  {week.focus}
                </Typography>
              </Box>
              <Typography
                variant="caption"
                color="text.secondary"
                sx={{ fontFamily: "monospace" }}
              >
                {formatDuration(
                  week.items.reduce((s, i) => s + i.hours_est, 0),
                )}
              </Typography>
            </Box>
            <CardContent>
              <Stack spacing={1}>
                {week.items.map((item, idx) => (
                  <Box
                    key={idx}
                    sx={{
                      display: "flex",
                      justifyContent: "space-between",
                      alignItems: "center",
                      p: 1,
                      pl: 1.5,
                      bgcolor: "action.hover",
                      borderRadius: 0.5,
                    }}
                  >
                    <Box
                      sx={{ display: "flex", gap: 1.25, alignItems: "center" }}
                    >
                      <Chip
                        label={item.kind}
                        size="small"
                        color="primary"
                        variant="outlined"
                        sx={{ height: 20, fontSize: 9, letterSpacing: 1 }}
                      />
                      <Typography
                        variant="caption"
                        color="text.secondary"
                        sx={{ fontFamily: "monospace" }}
                      >
                        {item.ref_id}
                      </Typography>
                    </Box>
                    <Typography
                      variant="caption"
                      color="text.secondary"
                      sx={{ fontFamily: "monospace" }}
                    >
                      {formatDuration(item.hours_est)}
                    </Typography>
                  </Box>
                ))}
              </Stack>
            </CardContent>
          </Card>
        ))}
      </Stack>

      <Box sx={{ mt: 3.5 }}>
        <Button variant="outlined" onClick={() => router.push("/progress")}>
          ← Back to progress
        </Button>
      </Box>
    </Box>
  );
}
