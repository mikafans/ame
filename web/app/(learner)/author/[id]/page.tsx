"use client";

import { use, useEffect, useMemo, useState } from "react";
import { useRouter } from "next/navigation";
import { ArrowRight, Check, Plus, Trash2 } from "lucide-react";
import { api } from "@/api/client";
import { MarkdownView } from "@/components/MarkdownView";
import { Button } from "@/components/ui/button";

type Question = {
  id: string;
  kind: string;
  prompt: string;
  tags: string[];
  payload: any;
  explanation?: string;
  deepDive?: string;
  source?: string;
  points: number;
  status: string;
  orderIndex: number;
};
type Assessment = {
  id: string;
  title: string;
  status: string;
  course?: string;
  description?: string;
  questions: Question[];
  updated_at?: string;
};
const kinds = ["mc", "tf", "short", "essay", "code"];
const kindLabels: Record<string, string> = {
  mc: "Multiple choice",
  tf: "True / false",
  short: "Short answer",
  essay: "Essay",
  code: "Code",
};

export default function AuthorStudioPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  const router = useRouter();
  const [assessment, setAssessment] = useState<Assessment | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [title, setTitle] = useState("");
  const [course, setCourse] = useState("");
  const [prompt, setPrompt] = useState("");
  const [explanation, setExplanation] = useState("");
  const [deepDive, setDeepDive] = useState("");
  const [source, setSource] = useState("");
  const [points, setPoints] = useState(1);
  const [tag, setTag] = useState("");
  const [payload, setPayload] = useState<any>({});
  const [picker, setPicker] = useState(false);
  const [saving, setSaving] = useState(false);
  const [publishing, setPublishing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    const { data } = await api.GET("/v1/assessments/{id}", {
      params: { path: { id } },
    });
    if (!data) return;
    const next = data as any as Assessment;
    setAssessment(next);
    setTitle(next.title ?? "");
    setCourse(next.course ?? "");
    setSelectedId((current) => current ?? next.questions[0]?.id ?? null);
  };
  useEffect(() => {
    load().catch(console.error);
  }, [id]);
  const selected =
    assessment?.questions.find((q) => q.id === selectedId) ?? null;
  useEffect(() => {
    if (!selected) return;
    setPrompt(selected.prompt ?? "");
    setExplanation(selected.explanation ?? "");
    setDeepDive(selected.deepDive ?? "");
    setSource(selected.source ?? "");
    setPoints(selected.points ?? 1);
    setTag(selected.tags?.join(", ") ?? "");
    setPayload(selected.payload ?? {});
  }, [selected]);

  const saveMetadata = async () => {
    await api.PATCH("/v1/assessments/{id}", {
      params: { path: { id } },
      body: { title, course: course || undefined },
    });
    await load();
  };
  const saveQuestion = async (nextPayload = payload) => {
    if (!selectedId) return;
    if (source && !/^https?:\/\//.test(source)) {
      setError("Reference URL must start with http:// or https://");
      return;
    }
    setError(null);
    setSaving(true);
    try {
      await api.PATCH("/v1/questions/{id}", {
        params: { path: { id: selectedId } },
        body: {
          prompt,
          explanation: explanation || undefined,
          deepDive: deepDive || undefined,
          source: source || undefined,
          points,
          tags: tag
            .split(",")
            .map((v) => v.trim())
            .filter(Boolean),
          payload: nextPayload,
        },
      });
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Could not save question");
    } finally {
      setSaving(false);
    }
  };
  const addQuestion = async (kind: string) => {
    setPicker(false);
    setSaving(true);
    try {
      const { data } = await api.POST("/v1/assessments/{id}/questions", {
        params: { path: { id } },
        body: { kind: kind as any, prompt: "" },
      });
      await load();
      if ((data as any)?.questionId) setSelectedId((data as any).questionId);
    } finally {
      setSaving(false);
    }
  };
  const deleteQuestion = async (questionId: string) => {
    await api.DELETE("/v1/assessments/{id}/questions/{question_id}", {
      params: { path: { id, question_id: questionId } },
    });
    setSelectedId(null);
    await load();
  };
  const publish = async () => {
    setPublishing(true);
    setError(null);
    const { error: publishError } = await api.PATCH("/v1/assessments/{id}", {
      params: { path: { id } },
      body: { status: "active" },
    });
    setPublishing(false);
    if (publishError)
      setError(
        typeof publishError === "object" && "message" in publishError
          ? String((publishError as any).message)
          : "Could not publish assessment",
      );
    else router.push(`/assessments/${id}/preview`);
  };

  const incomplete =
    assessment?.questions.filter((q) => !q.prompt?.trim() || !q.points)
      .length ?? 0;
  const totalPoints =
    assessment?.questions.reduce((sum, q) => sum + (q.points || 0), 0) ?? 0;
  const mcOptions = Array.isArray(payload.options) ? payload.options : [];
  const setMcOption = (index: number, value: string) =>
    setPayload({
      ...payload,
      options: mcOptions.map((option: string, i: number) =>
        i === index ? value : option,
      ),
    });

  if (!assessment)
    return (
      <div className="grid min-h-[60vh] place-items-center text-sm text-muted-foreground">
        Loading assessment…
      </div>
    );
  return (
    <main className="mx-auto max-w-[1500px] px-5 py-8 sm:px-10">
      <header className="mb-6 flex flex-col justify-between gap-4 lg:flex-row lg:items-start">
        <div>
          <p className="mb-1 font-mono text-xs uppercase tracking-widest text-muted-foreground">
            Editing draft · {course || "untitled course"}
          </p>
          <h1 className="text-2xl font-semibold tracking-tight">
            Author studio
          </h1>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button variant="outline" disabled>
            Import
          </Button>
          <Button
            variant="outline"
            onClick={() => router.push(`/assessments/${id}/preview`)}
          >
            Preview
          </Button>
          <Button variant="outline" onClick={() => saveMetadata()}>
            Save draft
          </Button>
          <Button
            onClick={publish}
            disabled={publishing || assessment.status === "active"}
          >
            {publishing ? "Publishing…" : "Publish"}
            <ArrowRight />
          </Button>
        </div>
      </header>
      {error && (
        <div
          role="alert"
          className="mb-5 rounded-lg border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive"
        >
          {error}
        </div>
      )}

      <section className="mb-5 rounded-xl border border-border bg-card">
        <div className="grid gap-4 border-b border-border p-5 sm:grid-cols-3">
          <Field label="Title">
            <input
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              onBlur={saveMetadata}
            />
          </Field>
          <Field label="Course">
            <input
              value={course}
              onChange={(e) => setCourse(e.target.value)}
              onBlur={saveMetadata}
              placeholder="Optional course or track"
            />
          </Field>
          <div className="flex items-end text-sm text-muted-foreground">
            {assessment.questions.length} questions · {totalPoints} pts
          </div>
        </div>
        <div className="flex flex-wrap gap-4 px-5 py-3 text-xs text-muted-foreground">
          <span
            className={
              title.trim() && assessment.questions.length
                ? "text-emerald-600 dark:text-emerald-300"
                : ""
            }
          >
            {title.trim() && assessment.questions.length ? "✓" : "✗"} Outline
            complete
          </span>
          <span
            className={
              incomplete === 0
                ? "text-emerald-600 dark:text-emerald-300"
                : "text-amber-600 dark:text-amber-300"
            }
          >
            {incomplete === 0
              ? "✓ All questions ready"
              : `${incomplete} question${incomplete === 1 ? "" : "s"} need review`}
          </span>
        </div>
      </section>

      <div className="grid min-h-[600px] gap-5 lg:grid-cols-[280px_minmax(0,1fr)_240px]">
        <section className="overflow-hidden rounded-xl border border-border bg-card">
          <div className="flex items-center justify-between border-b border-border px-4 py-3">
            <p className="font-mono text-xs uppercase tracking-widest text-muted-foreground">
              Questions
            </p>
            <Button
              size="sm"
              variant="outline"
              onClick={() => setPicker(!picker)}
            >
              <Plus /> Add
            </Button>
          </div>
          {picker && (
            <div className="border-b border-border bg-muted/30 p-3">
              <p className="mb-2 text-xs text-muted-foreground">Choose type</p>
              <div className="flex flex-wrap gap-2">
                {kinds.map((kind) => (
                  <Button
                    key={kind}
                    size="sm"
                    variant="outline"
                    onClick={() => addQuestion(kind)}
                  >
                    {kind === "tf" ? "T/F" : kindLabels[kind]}
                  </Button>
                ))}
              </div>
            </div>
          )}
          <div>
            {assessment.questions.length === 0 ? (
              <p className="p-8 text-center text-sm text-muted-foreground">
                No questions yet.
              </p>
            ) : (
              assessment.questions.map((question, index) => (
                <button
                  key={question.id}
                  type="button"
                  aria-label={`Question ${index + 1}: ${question.prompt || "Untitled question"}`}
                  onClick={() => setSelectedId(question.id)}
                  className={`flex w-full items-start gap-3 border-b border-border px-4 py-3 text-left transition hover:bg-muted/40 ${question.id === selectedId ? "bg-primary/10" : ""}`}
                >
                  <span className="font-mono text-xs text-muted-foreground">
                    {String(index + 1).padStart(2, "0")}
                  </span>
                  <span className="min-w-0 flex-1">
                    <span className="block truncate text-sm font-medium">
                      {question.prompt || "Untitled question"}
                    </span>
                    <span className="mt-1 block text-xs text-muted-foreground">
                      {kindLabels[question.kind] ?? question.kind} ·{" "}
                      {question.points} pt
                    </span>
                  </span>
                  {!question.prompt && (
                    <span className="text-amber-500">!</span>
                  )}
                </button>
              ))
            )}
          </div>
        </section>

        <section className="rounded-xl border border-border bg-card p-6">
          <div className="mb-6 flex items-start justify-between gap-4">
            <div>
              <p className="font-mono text-xs uppercase tracking-widest text-muted-foreground">
                Question editor
              </p>
              <h2 className="mt-1 text-lg font-semibold">
                {selected
                  ? `${kindLabels[selected.kind] ?? selected.kind} question`
                  : "Select a question"}
              </h2>
            </div>
            {selected && (
              <Button
                size="sm"
                variant="destructive"
                onClick={() => deleteQuestion(selected.id)}
              >
                <Trash2 /> Delete
              </Button>
            )}
          </div>
          {selected ? (
            <div className="space-y-5">
              <Field label="Question prompt">
                <textarea
                  rows={4}
                  value={prompt}
                  onChange={(e) => setPrompt(e.target.value)}
                  onBlur={() => saveQuestion()}
                />
              </Field>
              {selected.kind === "mc" && (
                <div>
                  <p className="mb-2 text-xs font-semibold uppercase tracking-widest text-muted-foreground">
                    Options · click to mark correct
                  </p>
                  <div className="space-y-2">
                    {mcOptions.map((option: string, index: number) => (
                      <div
                        key={index}
                        className={`flex items-center gap-2 rounded-lg border p-2 ${payload.correct_index === index ? "border-primary bg-primary/10" : "border-border"}`}
                      >
                        <button
                          type="button"
                          onClick={() =>
                            saveQuestion({ ...payload, correct_index: index })
                          }
                          className="grid size-6 shrink-0 place-items-center rounded-full border border-border"
                        >
                          {payload.correct_index === index && (
                            <Check className="size-3 text-primary" />
                          )}
                        </button>
                        <input
                          value={option}
                          onChange={(e) => setMcOption(index, e.target.value)}
                          onBlur={() => saveQuestion()}
                          className="min-w-0 flex-1 bg-transparent text-sm outline-none"
                          placeholder={`Option ${index + 1}`}
                        />
                      </div>
                    ))}
                  </div>
                </div>
              )}
              {selected.kind === "tf" && (
                <div>
                  <p className="mb-2 text-xs font-semibold uppercase tracking-widest text-muted-foreground">
                    Correct answer
                  </p>
                  <div className="flex gap-2">
                    {[true, false].map((value) => (
                      <button
                        type="button"
                        key={String(value)}
                        onClick={() =>
                          saveQuestion({ ...payload, correct: value })
                        }
                        className={`flex-1 rounded-lg border p-3 text-sm ${payload.correct === value ? "border-primary bg-primary/10" : "border-border"}`}
                      >
                        {value ? "True" : "False"}
                      </button>
                    ))}
                  </div>
                </div>
              )}
              <div className="grid gap-4 sm:grid-cols-3">
                <Field label="Points">
                  <input
                    type="number"
                    min="0"
                    value={points}
                    onChange={(e) => setPoints(Number(e.target.value))}
                    onBlur={() => saveQuestion()}
                  />
                </Field>
                <Field label="Tag">
                  <input
                    value={tag}
                    onChange={(e) => setTag(e.target.value)}
                    onBlur={() => saveQuestion()}
                    placeholder="rust, ownership"
                  />
                </Field>
                <Field label="Reference URL">
                  <input
                    value={source}
                    onChange={(e) => setSource(e.target.value)}
                    onBlur={() => saveQuestion()}
                    placeholder="https://…"
                  />
                </Field>
              </div>
              <Field label="Explanation shown after answering">
                <textarea
                  rows={3}
                  value={explanation}
                  onChange={(e) => setExplanation(e.target.value)}
                  onBlur={() => saveQuestion()}
                />
              </Field>
              <Field label="Deep Dive Study Notes">
                <textarea
                  rows={7}
                  value={deepDive}
                  onChange={(e) => setDeepDive(e.target.value)}
                  onBlur={() => saveQuestion()}
                  placeholder="# Study notes"
                />
              </Field>
              {(source || deepDive) && (
                <div className="rounded-lg border border-dashed border-border bg-muted/30 p-4">
                  <p className="mb-3 text-xs font-semibold uppercase tracking-widest text-muted-foreground">
                    Live preview
                  </p>
                  {source && /^https?:\/\//.test(source) && (
                    <a
                      className="mb-3 block text-sm text-primary underline"
                      href={source}
                      target="_blank"
                      rel="noreferrer"
                    >
                      {source}
                    </a>
                  )}
                  {deepDive && <MarkdownView content={deepDive} />}
                </div>
              )}
              {saving && (
                <p className="text-xs text-muted-foreground">Saving…</p>
              )}
            </div>
          ) : (
            <div className="grid min-h-96 place-items-center text-sm text-muted-foreground">
              Select a question to edit.
            </div>
          )}
        </section>

        <aside className="space-y-5">
          <InfoCard title="Distribution">
            <Stat
              label="Questions"
              value={String(assessment.questions.length)}
            />
            <Stat label="Total points" value={String(totalPoints)} />
            <Stat label="Status" value={assessment.status} />
          </InfoCard>
          <InfoCard title="Rubric">
            <pre className="whitespace-pre-wrap rounded-lg bg-muted p-3 text-xs leading-5 text-muted-foreground">{`criteria:\n  clarity: 0–2\n  evidence: 0–2\n  mechanism: 0–1`}</pre>
          </InfoCard>
        </aside>
      </div>
    </main>
  );
}

function Field({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <label className="grid gap-1.5 text-sm font-medium">
      <span className="text-xs uppercase tracking-wider text-muted-foreground">
        {label}
      </span>
      {children}
    </label>
  );
}
function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex justify-between border-b border-border py-2 text-sm last:border-0">
      <span className="text-muted-foreground">{label}</span>
      <span className="font-medium">{value}</span>
    </div>
  );
}
function InfoCard({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section className="rounded-xl border border-border bg-card p-5">
      <p className="mb-3 font-mono text-xs uppercase tracking-widest text-muted-foreground">
        {title}
      </p>
      {children}
    </section>
  );
}
