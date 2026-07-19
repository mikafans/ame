"use client";
import { CheckCircle2 } from "lucide-react";

interface Props {
  items: string[];
  kicker?: string;
  compact?: boolean;
  accentBars?: boolean;
}
export function LearningObjectives({
  items,
  kicker = "What you'll learn",
  compact = false,
  accentBars = true,
}: Props) {
  if (!items.length) return null;
  const overLimit = items.length > 6;
  if (compact)
    return (
      <div>
        <p className="mb-2 text-xs uppercase tracking-widest text-muted-foreground">
          {kicker}
        </p>
        <ul className="m-0 flex list-none flex-col gap-2 p-0">
          {items.map((it, i) => (
            <li key={i} className="flex items-start gap-2">
              <CheckCircle2
                size={16}
                className="mt-0.5 shrink-0 text-primary"
              />
              <span className="text-sm text-muted-foreground">{it}</span>
            </li>
          ))}
        </ul>
        {overLimit && (
          <p className="mt-2 text-xs text-amber-600 dark:text-amber-400">
            {items.length} objectives — consider splitting
          </p>
        )}
      </div>
    );
  return (
    <div className="rounded-lg border border-border bg-muted/40 p-5">
      <div className="mb-3 flex items-center gap-2">
        <p className="text-xs uppercase tracking-widest text-muted-foreground">
          {kicker}
        </p>
        {overLimit && (
          <p className="ml-auto text-xs text-amber-600 dark:text-amber-400">
            {items.length} objectives
          </p>
        )}
      </div>
      <ul className="m-0 grid list-none gap-3 p-0 sm:grid-cols-2">
        {items.map((it, i) => (
          <li
            key={i}
            className={`flex items-start gap-3 ${accentBars ? "border-l-2 border-primary pl-3" : ""}`}
          >
            <span className="shrink-0 pt-0.5 font-mono text-xs text-muted-foreground">
              {String(i + 1).padStart(2, "0")}
            </span>
            <span className="text-sm text-muted-foreground">{it}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}
