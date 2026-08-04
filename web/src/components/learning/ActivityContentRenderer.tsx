"use client";

import { useEffect, useState } from "react";
import { Check } from "lucide-react";
import { HighlightedCode } from "@/components/HighlightedCode";
import { MarkdownView, MermaidView } from "@/components/MarkdownView";
import { api } from "@/api/client";
import type { components } from "@/api/generated/schema.d.ts";

type TaskSubmission = components["schemas"]["TaskSubmissionResponse"];
type TaskRubric = components["schemas"]["TaskRubric"];

export type ActivityContent =
  | {
      type: "explanation";
      heading: string;
      body: string;
      keyPoints: string[];
    }
  | {
      type: "worked_example";
      heading: string;
      prompt: string;
      steps: string[];
      reflection: string;
    }
  | { type: "rich_text"; heading: string; body: string }
  | { type: "diagram"; title: string; source: string; altText: string }
  | {
      type: "code_example";
      title: string;
      language: string;
      code: string;
      explanation: string;
    }
  | {
      type: "scenario";
      context: string;
      prompt: string;
      options: { id: string; label: string }[];
      correctOptionId?: string;
      feedback?: { correct: string; incorrect: string };
    };

type UnknownContent = Record<string, unknown> & { type?: unknown };

function isNonEmptyString(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

function isStringList(value: unknown): value is string[] {
  return (
    Array.isArray(value) && value.length > 0 && value.every(isNonEmptyString)
  );
}

function parseContent(value: unknown): ActivityContent | null {
  if (!value || typeof value !== "object") return null;
  const content = value as UnknownContent;
  if (content.type === "explanation") {
    if (
      isNonEmptyString(content.heading) &&
      isNonEmptyString(content.body) &&
      isStringList(content.keyPoints)
    )
      return content as unknown as ActivityContent;
  }
  if (content.type === "worked_example") {
    if (
      isNonEmptyString(content.heading) &&
      isNonEmptyString(content.prompt) &&
      isNonEmptyString(content.reflection) &&
      isStringList(content.steps)
    )
      return content as unknown as ActivityContent;
  }
  if (content.type === "rich_text") {
    if (isNonEmptyString(content.heading) && isNonEmptyString(content.body))
      return content as unknown as ActivityContent;
  }
  if (content.type === "diagram") {
    if (
      isNonEmptyString(content.title) &&
      isNonEmptyString(content.source) &&
      isNonEmptyString(content.altText)
    )
      return content as unknown as ActivityContent;
  }
  if (content.type === "code_example") {
    if (
      isNonEmptyString(content.title) &&
      isNonEmptyString(content.language) &&
      isNonEmptyString(content.code) &&
      isNonEmptyString(content.explanation)
    )
      return content as unknown as ActivityContent;
  }
  if (content.type === "scenario") {
    if (
      isNonEmptyString(content.context) &&
      isNonEmptyString(content.prompt) &&
      Array.isArray(content.options) &&
      content.options.length >= 2 &&
      content.options.every(
        (option) =>
          option &&
          typeof option === "object" &&
          isNonEmptyString((option as { id?: unknown }).id) &&
          isNonEmptyString((option as { label?: unknown }).label),
      )
    )
      return content as unknown as ActivityContent;
  }
  return null;
}

export function ActivityContentRenderer({
  content,
  taskId,
  contentVersion = 1,
  rubric,
}: {
  content: unknown;
  taskId?: string;
  contentVersion?: number;
  rubric?: TaskRubric | null;
}) {
  const parsed = parseContent(content);
  const [selectedOption, setSelectedOption] = useState<string | null>(null);
  const [taskStatus, setTaskStatus] = useState<
    "idle" | "submitting" | "submitted" | "error"
  >("idle");
  const [taskSubmission, setTaskSubmission] = useState<TaskSubmission | null>(
    null,
  );
  const [projectResponse, setProjectResponse] = useState("");
  const [artifactContent, setArtifactContent] = useState("");

  useEffect(() => {
    if (!taskId || typeof window === "undefined") return;
    const submissionId = window.localStorage.getItem(
      `ame-task-submission:${taskId}`,
    );
    if (!submissionId) return;
    void api
      .GET("/api/v1/task-submissions/{submission_id}", {
        params: { path: { submission_id: submissionId } },
      })
      .then(({ data, response }) => {
        if (response.ok && data) {
          setTaskSubmission(data);
          setTaskStatus("submitted");
        }
      });
  }, [taskId]);

  if (!parsed) {
    const unknown = content as UnknownContent | null;
    const type = isNonEmptyString(unknown?.type) ? unknown.type : "unknown";
    return (
      <div className="mt-5 space-y-5">
        <div className="rounded-xl border border-dashed border-primary/40 bg-background/70 p-5">
          <p className="text-xs font-bold uppercase tracking-[0.12em] text-primary">
            Activity capability unavailable
          </p>
          <p className="mt-2 text-sm leading-6 text-muted-foreground">
            This activity uses the <code>{type}</code> format. The content is
            preserved, but this learner client does not support it yet.
          </p>
        </div>
        {rubric && <RubricSection rubric={rubric} />}
      </div>
    );
  }

  return (
    <div className="mt-5 space-y-5 border-t border-primary/20 pt-5">
      {parsed.type === "explanation" && (
        <>
          <CapabilityHeading label="Explanation" title={parsed.heading} />
          <MarkdownView content={parsed.body} />
          <ul className="space-y-2 rounded-xl border border-primary/20 bg-background/70 p-4 text-sm leading-6">
            {parsed.keyPoints.map((point) => (
              <li key={point} className="flex gap-2">
                <Check className="mt-1 size-4 shrink-0 text-primary" />
                <span>{point}</span>
              </li>
            ))}
          </ul>
        </>
      )}
      {parsed.type === "worked_example" && (
        <>
          <CapabilityHeading label="Worked example" title={parsed.heading} />
          <MarkdownView content={parsed.prompt} />
          <ol className="space-y-2 text-sm leading-6">
            {parsed.steps.map((step, index) => (
              <li key={step} className="flex gap-3">
                <span className="font-mono text-primary">{index + 1}.</span>
                <span>{step}</span>
              </li>
            ))}
          </ol>
          <p className="border-t border-primary/20 pt-4 text-sm leading-6 text-muted-foreground">
            {parsed.reflection}
          </p>
        </>
      )}
      {parsed.type === "rich_text" && (
        <>
          <CapabilityHeading label="Reading" title={parsed.heading} />
          <MarkdownView content={parsed.body} />
        </>
      )}
      {parsed.type === "diagram" && (
        <>
          <CapabilityHeading label="Diagram" title={parsed.title} />
          <MermaidView code={parsed.source} />
          <p className="text-sm text-muted-foreground">{parsed.altText}</p>
        </>
      )}
      {parsed.type === "code_example" && (
        <>
          <CapabilityHeading label="Code example" title={parsed.title} />
          <HighlightedCode code={parsed.code} language={parsed.language} />
          <MarkdownView content={parsed.explanation} />
        </>
      )}
      {parsed.type === "scenario" && (
        <>
          <CapabilityHeading label="Scenario" title={parsed.prompt} />
          <p className="rounded-xl border border-primary/20 bg-background/70 p-4 text-sm leading-6">
            {parsed.context}
          </p>
          <div className="grid gap-2 sm:grid-cols-2">
            {parsed.options.map((option) => (
              <button
                key={option.id}
                type="button"
                onClick={() => setSelectedOption(option.id)}
                className={`rounded-xl border p-4 text-left text-sm transition-colors ${
                  selectedOption === option.id
                    ? "border-primary bg-primary/10"
                    : "border-border hover:border-primary/60"
                }`}
              >
                {option.label}
              </button>
            ))}
          </div>
          {selectedOption && (
            <div className="space-y-3">
              {parsed.correctOptionId && parsed.feedback && (
                <p className="rounded-xl border border-primary/20 bg-background/70 p-4 text-sm leading-6">
                  {selectedOption === parsed.correctOptionId
                    ? parsed.feedback.correct
                    : parsed.feedback.incorrect}
                </p>
              )}
              <p className="text-sm text-muted-foreground">
                {taskSubmission?.status === "reviewed" &&
                typeof taskSubmission.score === "number"
                  ? `Reviewed · ${Math.round(taskSubmission.score * 100)}%`
                  : taskSubmission?.status === "rejected"
                    ? "Needs another attempt · this task was rejected."
                    : taskSubmission?.reviewStatus === "pending"
                      ? "Submitted · awaiting reviewer feedback."
                      : taskStatus === "submitted"
                        ? "Task submitted. Your answer is now part of this journey."
                        : "Submit your choice as evidence of applying the concept."}
              </p>
              {taskSubmission?.feedback != null && (
                <p className="rounded-xl border border-primary/20 bg-background/70 p-4 text-sm leading-6">
                  {typeof taskSubmission.feedback === "object" &&
                  taskSubmission.feedback !== null &&
                  "note" in taskSubmission.feedback
                    ? String(taskSubmission.feedback.note)
                    : "Reviewer feedback is available for this submission."}
                </p>
              )}
              {taskSubmission?.status === "rejected" && taskId && (
                <button
                  className="rounded-full bg-primary px-4 py-2 text-sm font-medium text-primary-foreground"
                  onClick={async () => {
                    setTaskStatus("submitting");
                    const revised = await api.PATCH(
                      "/api/v1/task-submissions/{submission_id}/revise",
                      {
                        params: {
                          path: { submission_id: taskSubmission.id },
                        },
                        body: {
                          response: { optionId: selectedOption },
                          artifacts: [],
                        },
                      },
                    );
                    if (!revised.response.ok || !revised.data) {
                      setTaskStatus("error");
                      return;
                    }
                    const submitted = await api.POST(
                      "/api/v1/task-submissions/{submission_id}/submit",
                      {
                        params: {
                          path: { submission_id: revised.data.id },
                        },
                      },
                    );
                    if (submitted.response.ok && submitted.data) {
                      setTaskSubmission(submitted.data);
                      window.localStorage.setItem(
                        `ame-task-submission:${taskId}`,
                        submitted.data.id,
                      );
                      setTaskStatus("submitted");
                    } else {
                      setTaskStatus("error");
                    }
                  }}
                  type="button"
                >
                  Revise and resubmit
                </button>
              )}
              {taskId && taskStatus !== "submitted" && (
                <button
                  type="button"
                  disabled={taskStatus === "submitting"}
                  onClick={async () => {
                    setTaskStatus("submitting");
                    const started = await api.POST(
                      "/api/v1/tasks/{task_id}/submissions",
                      {
                        params: { path: { task_id: taskId } },
                        body: {
                          contentVersion,
                          response: { optionId: selectedOption },
                          evaluationMethod: "self_review",
                        },
                      },
                    );
                    if (!started.response.ok || !started.data) {
                      setTaskStatus("error");
                      return;
                    }
                    const submitted = await api.POST(
                      "/api/v1/task-submissions/{submission_id}/submit",
                      { params: { path: { submission_id: started.data.id } } },
                    );
                    if (submitted.response.ok && submitted.data) {
                      setTaskSubmission(submitted.data);
                      window.localStorage.setItem(
                        `ame-task-submission:${taskId}`,
                        submitted.data.id,
                      );
                      setTaskStatus("submitted");
                    } else {
                      setTaskStatus("error");
                    }
                  }}
                  className="rounded-full bg-primary px-4 py-2 text-sm font-medium text-primary-foreground disabled:opacity-60"
                >
                  {taskStatus === "submitting" ? "Submitting…" : "Submit task"}
                </button>
              )}
              {taskStatus === "error" && (
                <p className="text-sm text-destructive">
                  Could not submit this task. Try again.
                </p>
              )}
            </div>
          )}
        </>
      )}
      {rubric && <RubricSection rubric={rubric} />}
      {rubric && taskId && parsed.type !== "scenario" && (
        <div className="space-y-3 rounded-xl border border-primary/20 bg-background/70 p-4">
          <label
            className="block text-sm font-medium"
            htmlFor={`project-${taskId}`}
          >
            Project response
          </label>
          <textarea
            className="min-h-28 w-full rounded-xl border border-input bg-background p-3 text-sm"
            id={`project-${taskId}`}
            onChange={(event) => setProjectResponse(event.target.value)}
            placeholder="Explain your approach, decisions, and result…"
            value={projectResponse}
          />
          <label
            className="block text-sm font-medium"
            htmlFor={`artifact-${taskId}`}
          >
            Supporting artifact
          </label>
          <textarea
            className="min-h-24 w-full rounded-xl border border-input bg-background p-3 font-mono text-sm"
            id={`artifact-${taskId}`}
            onChange={(event) => setArtifactContent(event.target.value)}
            placeholder="Paste a report, data sample, or code excerpt…"
            value={artifactContent}
          />
          {taskSubmission?.reviewStatus === "pending" && (
            <p className="text-sm text-muted-foreground">
              Submitted · awaiting rubric review.
            </p>
          )}
          {taskSubmission?.feedback != null && (
            <pre className="whitespace-pre-wrap rounded-lg bg-muted p-3 text-sm">
              {JSON.stringify(taskSubmission.feedback, null, 2)}
            </pre>
          )}
          {taskSubmission?.rubricScores?.map((score) => (
            <p className="text-sm" key={score.criterionId}>
              {score.criterionId}: {score.points} pts · {score.feedback}
            </p>
          ))}
          <button
            className="rounded-full bg-primary px-4 py-2 text-sm font-medium text-primary-foreground disabled:opacity-60"
            disabled={taskStatus === "submitting" || !projectResponse.trim()}
            onClick={async () => {
              setTaskStatus("submitting");
              const artifacts = artifactContent.trim()
                ? [
                    {
                      kind: "text",
                      name: "learner-artifact.txt",
                      mediaType: "text/plain",
                      content: artifactContent,
                    },
                  ]
                : [];
              const result =
                taskSubmission?.status === "rejected"
                  ? await api.PATCH(
                      "/api/v1/task-submissions/{submission_id}/revise",
                      {
                        params: {
                          path: { submission_id: taskSubmission.id },
                        },
                        body: {
                          response: { text: projectResponse },
                          artifacts,
                        },
                      },
                    )
                  : await api.POST("/api/v1/tasks/{task_id}/submissions", {
                      params: { path: { task_id: taskId } },
                      body: {
                        contentVersion,
                        response: { text: projectResponse },
                        artifacts,
                        evaluationMethod: "agent",
                      },
                    });
              if (!result.response.ok || !result.data) {
                setTaskStatus("error");
                return;
              }
              const submitted = await api.POST(
                "/api/v1/task-submissions/{submission_id}/submit",
                {
                  params: {
                    path: { submission_id: result.data.id },
                  },
                },
              );
              if (submitted.response.ok && submitted.data) {
                setTaskSubmission(submitted.data);
                window.localStorage.setItem(
                  `ame-task-submission:${taskId}`,
                  submitted.data.id,
                );
                setTaskStatus("submitted");
              } else {
                setTaskStatus("error");
              }
            }}
            type="button"
          >
            {taskSubmission?.status === "rejected"
              ? "Revise and resubmit"
              : "Submit for rubric review"}
          </button>
        </div>
      )}
    </div>
  );
}

function RubricSection({ rubric }: { rubric: TaskRubric }) {
  return (
    <div className="space-y-3 border-t border-primary/20 pt-5">
      <p className="text-xs font-bold uppercase tracking-[0.12em] text-primary">
        Scoring rubric
      </p>
      {typeof rubric.passingScore === "number" && (
        <p className="text-sm text-muted-foreground">
          {`Passing score: ${Math.round(rubric.passingScore * 100)}%`}
        </p>
      )}
      <ul className="space-y-2 rounded-xl border border-primary/20 bg-background/70 p-4 text-sm leading-6">
        {rubric.criteria.map((criterion) => (
          <li
            key={criterion.id}
            className="flex items-start justify-between gap-3"
          >
            <span>
              {criterion.description}
              {criterion.required && (
                <span className="ml-2 text-xs font-semibold uppercase tracking-wide text-primary">
                  Required
                </span>
              )}
            </span>
            <span className="shrink-0 font-mono text-muted-foreground">
              {`${criterion.maxPoints} pts`}
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}

function CapabilityHeading({ label, title }: { label: string; title: string }) {
  return (
    <div>
      <p className="text-xs font-bold uppercase tracking-[0.12em] text-primary">
        {label}
      </p>
      <h3 className="mt-2 text-lg font-semibold">{title}</h3>
    </div>
  );
}
