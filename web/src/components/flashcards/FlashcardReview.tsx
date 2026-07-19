"use client";
import { useEffect, useState } from "react";
import {
  deriveBack,
  hasModelAnswer,
  type FlashQuestion,
} from "@/lib/flashcards";
import { HighlightedCode } from "@/components/HighlightedCode";
import { Button } from "@/components/ui/button";

const KIND_LABEL: Record<string, string> = {
  mc: "Multiple choice",
  tf: "True / false",
  short: "Short answer",
  essay: "Essay",
  code: "Code",
};
interface Props {
  question: FlashQuestion;
  index: number;
  total: number;
  onRate: (knew: boolean) => void;
  onSkip: () => void;
  onFinish?: () => void;
}
export function FlashcardReview({
  question,
  index,
  total,
  onRate,
  onSkip,
  onFinish,
}: Props) {
  const [revealed, setRevealed] = useState(false);
  const back = deriveBack(question);
  useEffect(() => setRevealed(false), [question.id]);
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === " ") {
        e.preventDefault();
        setRevealed(true);
      } else if ((e.key === "s" || e.key === "S") && index + 1 < total)
        onSkip();
      else if (revealed && (e.key === "2" || e.key === "ArrowRight"))
        onRate(true);
      else if (revealed && (e.key === "1" || e.key === "ArrowLeft"))
        onRate(false);
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [revealed, onRate, onSkip, index, total]);
  const snippet =
    question.code_snippet && typeof question.code_snippet === "object"
      ? (question.code_snippet as { code?: string }).code
      : undefined;
  return (
    <div className="max-w-[680px]">
      <div className="mb-3 flex items-center justify-between">
        <span className="rounded-full border border-border px-2 py-1 text-xs">
          {KIND_LABEL[question.kind] ?? question.kind}
        </span>
        <span className="font-mono text-xs text-muted-foreground">
          {index + 1} / {total}
        </span>
      </div>
      <article className="min-h-60 rounded-lg border border-border p-6 sm:p-8">
        <h2 className="text-lg font-normal leading-7">{question.prompt}</h2>
        {snippet && (
          <div className="mt-4">
            <HighlightedCode
              code={snippet}
              language={(question.payload?.language as string) ?? "python"}
            />
          </div>
        )}
        {revealed && (
          <div className="mt-6 border-t border-border pt-6">
            {back.answer !== null && (
              <>
                <p className="mb-1 text-xs tracking-wide text-muted-foreground">
                  Answer
                </p>
                <p className="mb-4 font-medium">{back.answer}</p>
              </>
            )}
            {back.explanation && (
              <>
                <p className="mb-1 text-xs tracking-wide text-muted-foreground">
                  Explanation
                </p>
                <p className="text-sm text-muted-foreground">
                  {back.explanation}
                </p>
              </>
            )}
            {!hasModelAnswer(back) && (
              <p className="text-sm text-muted-foreground">
                No model answer provided.
              </p>
            )}
          </div>
        )}
      </article>
      <div className="mt-5 flex flex-wrap items-center justify-between gap-3">
        {!revealed ? (
          <Button onClick={() => setRevealed(true)}>Show answer (Space)</Button>
        ) : (
          <div className="flex gap-3">
            <Button
              variant="outline"
              className="border-red-500/50 text-red-600 hover:bg-red-500/10"
              onClick={() => onRate(false)}
            >
              Missed it (1)
            </Button>
            <Button
              className="bg-emerald-600 text-white hover:bg-emerald-700"
              onClick={() => onRate(true)}
            >
              Got it (2)
            </Button>
          </div>
        )}
        <div className="flex gap-2">
          {index + 1 < total && (
            <Button
              variant="ghost"
              className="text-muted-foreground"
              onClick={onSkip}
            >
              Skip (S)
            </Button>
          )}
          {onFinish && (
            <Button
              variant="ghost"
              className="text-muted-foreground"
              onClick={onFinish}
            >
              Finish
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
