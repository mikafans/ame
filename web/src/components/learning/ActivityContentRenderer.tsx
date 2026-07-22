"use client";

import { useState } from "react";
import { Check } from "lucide-react";
import { HighlightedCode } from "@/components/HighlightedCode";
import { MarkdownView, MermaidView } from "@/components/MarkdownView";
import { api } from "@/api/client";

export type ActivityContent =
  | {
      type: "explanation";
      heading: string;
      body: string;
      key_points: string[];
    }
  | {
      type: "worked_example";
      heading: string;
      prompt: string;
      steps: string[];
      reflection: string;
    }
  | { type: "rich_text"; heading: string; body: string }
  | { type: "diagram"; title: string; source: string; alt_text: string }
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
      isStringList(content.key_points)
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
      isNonEmptyString(content.alt_text)
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
}: {
  content: unknown;
  taskId?: string;
  contentVersion?: number;
}) {
  const parsed = parseContent(content);
  const [selectedOption, setSelectedOption] = useState<string | null>(null);
  const [taskStatus, setTaskStatus] = useState<
    "idle" | "submitting" | "submitted" | "error"
  >("idle");

  if (!parsed) {
    const unknown = content as UnknownContent | null;
    const type = isNonEmptyString(unknown?.type) ? unknown.type : "unknown";
    return (
      <div className="mt-5 rounded-xl border border-dashed border-primary/40 bg-background/70 p-5">
        <p className="text-xs font-bold uppercase tracking-[0.12em] text-primary">
          Activity capability unavailable
        </p>
        <p className="mt-2 text-sm leading-6 text-muted-foreground">
          This activity uses the <code>{type}</code> format. The content is
          preserved, but this learner client does not support it yet.
        </p>
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
            {parsed.key_points.map((point) => (
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
          <p className="text-sm text-muted-foreground">{parsed.alt_text}</p>
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
              <p className="text-sm text-muted-foreground">
                {taskStatus === "submitted"
                  ? "Task submitted. Your answer is now part of this journey."
                  : "Submit your choice as evidence of applying the concept."}
              </p>
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
                    setTaskStatus(
                      submitted.response.ok ? "submitted" : "error",
                    );
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
