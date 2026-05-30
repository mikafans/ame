export type Kind = "mc" | "tf" | "short" | "essay" | "code";

export interface FlashQuestion {
  id: string;
  kind: Kind;
  prompt: string;
  payload: Record<string, unknown>;
  explanation: string | null;
  code_snippet?: unknown;
}

export interface CardBack {
  answer: string | null;
  explanation: string | null;
}

export function deriveBack(q: FlashQuestion): CardBack {
  const p = q.payload ?? {};
  let answer: string | null = null;

  switch (q.kind) {
    case "mc": {
      const options = p.options as unknown;
      const idx = p.correct_index as unknown;
      if (Array.isArray(options) && typeof idx === "number") {
        answer = (options[idx] as string) ?? null;
      }
      break;
    }
    case "tf": {
      if (typeof p.correct === "boolean") answer = p.correct ? "True" : "False";
      break;
    }
    case "short": {
      const accepted = p.accepted as unknown;
      if (Array.isArray(accepted) && accepted.length > 0) {
        answer = (accepted as string[]).join(", ");
      }
      break;
    }
    case "essay":
    case "code":
      answer = null;
      break;
  }

  return { answer, explanation: q.explanation ?? null };
}

export function hasModelAnswer(back: CardBack): boolean {
  if (back.answer !== null && back.answer.trim() !== "") return true;
  return back.explanation !== null && back.explanation.trim() !== "";
}

export function filterByTypes<T extends { kind: Kind }>(
  items: T[],
  types: string[],
): T[] {
  if (types.length === 0) return items;
  return items.filter((i) => types.includes(i.kind));
}

export function shuffle<T>(items: T[], rng: () => number = Math.random): T[] {
  const a = [...items];
  for (let i = a.length - 1; i > 0; i--) {
    const j = Math.floor(rng() * (i + 1));
    [a[i], a[j]] = [a[j], a[i]];
  }
  return a;
}

export function buildDeck<T>(
  items: T[],
  count: number,
  rng: () => number = Math.random,
): T[] {
  return shuffle(items, rng).slice(0, count);
}
