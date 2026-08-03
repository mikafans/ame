import { describe, expect, it } from "bun:test";
import React from "react";
import { renderToString } from "react-dom/server";
import { AssessmentResultFeedback } from "./AssessmentResultFeedback";

function attempt(overrides: Record<string, unknown> = {}) {
  return {
    id: "attempt-1",
    assessmentId: "assessment-1",
    assessmentVersion: 1,
    status: "graded",
    assessmentMode: "graded",
    reviewStatus: "not_required",
    responses: {},
    items: [
      {
        assessmentItemId: "item-1",
        questionVersionId: "question-1",
        response: "event_time",
        correctness: 1,
        awardedPoints: 1,
        evaluationStatus: "correct",
      },
    ],
    score: 1,
    awardedPoints: 1,
    maxPoints: 1,
    createdAt: "2026-07-22T00:00:00Z",
    submittedAt: "2026-07-22T00:01:00Z",
    ...overrides,
  } as never;
}

describe("AssessmentResultFeedback", () => {
  it("renders a graded score and item result", () => {
    const html = renderToString(
      <AssessmentResultFeedback attempt={attempt()} />,
    );
    expect(html).toContain("Graded · 100%");
    expect(html).toContain("correct");
  });

  it("explains pending review instead of inventing a score", () => {
    const html = renderToString(
      <AssessmentResultFeedback
        attempt={attempt({
          status: "submitted",
          score: null,
          reviewStatus: "pending",
        })}
      />,
    );
    expect(html).toContain("Submitted · pending review");
    expect(html).toContain("reviewer will add feedback");
    expect(html).not.toContain("Graded ·");
  });

  it("shows immutable answer feedback after a formative result", () => {
    const html = renderToString(
      <AssessmentResultFeedback
        attempt={attempt({
          items: [
            {
              assessmentItemId: "item-1",
              questionVersionId: "question-1",
              response: "processing_time",
              correctness: 0,
              awardedPoints: 0,
              evaluationStatus: "incorrect",
              explanation: "Event time is when the click occurred.",
              rationale:
                "This distinguishes delayed records from processing delay.",
            },
          ],
        })}
      />,
    );
    expect(html).toContain("Event time is when the click occurred.");
    expect(html).toContain("Why this matters:");
  });
});
