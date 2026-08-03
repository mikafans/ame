import type { components } from "@/api/generated/schema.d.ts";

type Attempt = components["schemas"]["AttemptResponse"];

export function AssessmentResultFeedback({ attempt }: { attempt: Attempt }) {
  if (attempt.status !== "graded" && attempt.status !== "submitted") {
    return null;
  }

  const pendingReview =
    attempt.status === "submitted" || attempt.reviewStatus === "pending";
  const score =
    typeof attempt.score === "number" ? Math.round(attempt.score * 100) : null;

  return (
    <section
      aria-label="Assessment result"
      className="mt-4 space-y-3 border-t border-primary/20 pt-4"
    >
      <div>
        <p className="text-xs font-bold uppercase tracking-[0.12em] text-primary">
          Assessment result
        </p>
        <p className="mt-1 text-sm font-medium text-foreground">
          {pendingReview
            ? "Submitted · pending review"
            : `Graded · ${score ?? 0}%`}
        </p>
        {pendingReview && (
          <p className="mt-1 text-sm text-muted-foreground">
            A reviewer will add feedback before this result contributes to
            mastery.
          </p>
        )}
      </div>
      {attempt.items.length > 0 && (
        <ul className="space-y-2 text-sm">
          {attempt.items.map((item) => {
            const isCorrect = item.evaluationStatus === "correct";
            return (
              <li
                key={item.assessmentItemId}
                className="rounded-lg bg-background/60 px-3 py-2"
              >
                <div className="flex items-center justify-between gap-4">
                  <span>Question result</span>
                  <span
                    className={
                      isCorrect
                        ? "font-semibold text-primary"
                        : "font-semibold text-destructive"
                    }
                  >
                    {item.evaluationStatus.replaceAll("_", " ")}
                  </span>
                </div>
                {(item.explanation || item.rationale) && (
                  <div className="mt-2 space-y-1 text-muted-foreground">
                    {item.explanation && <p>{item.explanation}</p>}
                    {item.rationale && (
                      <p className="text-xs">
                        Why this matters: {item.rationale}
                      </p>
                    )}
                  </div>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
