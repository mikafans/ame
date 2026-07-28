"use client";

import { useEffect, useState } from "react";
import { api } from "@/api/client";
import { Button } from "@/components/ui/button";

type VariantKind = "explanation" | "example" | "difficulty";
type Variant = {
  id: string;
  variantKind: VariantKind;
  recommendationReason: string;
  requestedDifficulty?: string | null;
  status: "requested" | "running" | "review_required" | "published" | "failed";
  error?: unknown;
  content?: unknown;
};

export function ActivityVariants({
  activityId,
  objectiveId,
}: {
  activityId: string;
  objectiveId: string;
}) {
  const [variants, setVariants] = useState<Variant[]>([]);
  const [requesting, setRequesting] = useState<VariantKind | null>(null);

  async function reload() {
    const result = await api.GET("/api/v1/learning-variants", {
      params: { query: { activityId } },
    });
    if (result.response.ok && result.data)
      setVariants(result.data as Variant[]);
  }

  useEffect(() => {
    void reload();
  }, [activityId]);

  async function request(kind: VariantKind) {
    setRequesting(kind);
    const reason =
      kind === "difficulty"
        ? "I am ready to test transfer with a harder variant."
        : `I need another ${kind} to connect this objective to what I know.`;
    const result = await api.POST("/api/v1/learning-variants", {
      body: {
        sourceActivityId: activityId,
        objectiveId,
        variantKind: kind,
        recommendationReason: reason,
        requestedDifficulty: kind === "difficulty" ? "harder" : undefined,
        retryKey: crypto.randomUUID(),
      },
    });
    if (result.response.ok) await reload();
    setRequesting(null);
  }

  return (
    <aside
      className="mt-6 rounded-2xl border border-border p-5"
      data-testid="activity-variants"
    >
      <h3 className="font-semibold">Try another angle</h3>
      <p className="mt-1 text-xs text-muted-foreground">
        Variants do not change mastery until you complete reviewed work.
      </p>
      <div className="mt-3 flex flex-wrap gap-2">
        {(["explanation", "example", "difficulty"] as const).map((kind) => (
          <Button
            disabled={requesting !== null}
            key={kind}
            onClick={() => void request(kind)}
            size="sm"
            variant="outline"
          >
            {kind === "difficulty" ? "Make it harder" : `Another ${kind}`}
          </Button>
        ))}
      </div>
      <div className="mt-4 space-y-3">
        {variants.map((variant) => (
          <article className="rounded-xl bg-muted p-4 text-sm" key={variant.id}>
            <p className="font-medium">
              {variant.variantKind.replaceAll("_", " ")} ·{" "}
              {variant.content && variant.status === "published"
                ? "available"
                : variant.status.replaceAll("_", " ")}
            </p>
            <p className="mt-1 text-xs text-muted-foreground">
              Why: {variant.recommendationReason}
            </p>
            {variant.status === "failed" && (
              <div className="mt-2">
                <p className="text-xs text-destructive">
                  Generation failed. You can request a fresh retry.
                </p>
                <Button
                  className="mt-2"
                  onClick={() => void request(variant.variantKind)}
                  size="sm"
                  variant="outline"
                >
                  Retry
                </Button>
              </div>
            )}
            {variant.content !== undefined &&
              variant.content !== null &&
              variant.status === "published" && (
                <pre className="mt-3 whitespace-pre-wrap rounded-lg bg-background p-3 text-xs">
                  {JSON.stringify(variant.content, null, 2)}
                </pre>
              )}
          </article>
        ))}
      </div>
    </aside>
  );
}
