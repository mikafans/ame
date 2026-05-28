import { describe, it, expect } from "bun:test";
import {
  deriveBack,
  hasModelAnswer,
  filterByTypes,
  buildDeck,
  type FlashQuestion,
} from "./flashcards";

function q(partial: Partial<FlashQuestion>): FlashQuestion {
  return {
    id: "00000000-0000-0000-0000-000000000000",
    kind: "mc",
    prompt: "Q?",
    payload: {},
    explanation: null,
    ...partial,
  };
}

describe("deriveBack", () => {
  it("mc → option at correct_index", () => {
    const back = deriveBack(
      q({ kind: "mc", payload: { options: ["a", "b", "c"], correct_index: 1 } }),
    );
    expect(back.answer).toBe("b");
  });

  it("tf → True/False string", () => {
    expect(deriveBack(q({ kind: "tf", payload: { correct: true } })).answer).toBe("True");
    expect(deriveBack(q({ kind: "tf", payload: { correct: false } })).answer).toBe("False");
  });

  it("short → accepted joined", () => {
    const back = deriveBack(q({ kind: "short", payload: { accepted: ["x", "y"] } }));
    expect(back.answer).toBe("x, y");
  });

  it("essay/code → no crisp answer, explanation passed through", () => {
    const back = deriveBack(q({ kind: "essay", explanation: "discuss tradeoffs" }));
    expect(back.answer).toBeNull();
    expect(back.explanation).toBe("discuss tradeoffs");
  });

  it("malformed mc payload → null answer", () => {
    expect(deriveBack(q({ kind: "mc", payload: {} })).answer).toBeNull();
  });
});

describe("hasModelAnswer", () => {
  it("true when an answer exists", () => {
    expect(hasModelAnswer({ answer: "b", explanation: null })).toBe(true);
  });
  it("true when only explanation exists", () => {
    expect(hasModelAnswer({ answer: null, explanation: "because" })).toBe(true);
  });
  it("false when neither", () => {
    expect(hasModelAnswer({ answer: null, explanation: "  " })).toBe(false);
  });
});

describe("filterByTypes", () => {
  const items = [q({ kind: "mc" }), q({ kind: "tf" }), q({ kind: "essay" })];
  it("empty selection keeps all", () => {
    expect(filterByTypes(items, [])).toHaveLength(3);
  });
  it("keeps only selected kinds", () => {
    expect(filterByTypes(items, ["mc", "tf"]).map((x) => x.kind)).toEqual(["mc", "tf"]);
  });
});

describe("buildDeck", () => {
  const items = Array.from({ length: 100 }, (_, i) => q({ id: String(i) }));
  it("slices to count", () => {
    expect(buildDeck(items, 30)).toHaveLength(30);
  });
  it("returns all when fewer than count", () => {
    expect(buildDeck(items.slice(0, 5), 30)).toHaveLength(5);
  });
  it("is a permutation (no dropped/duplicated items) for full deck", () => {
    const ids = buildDeck(items, 100).map((x) => x.id).sort();
    expect(ids).toEqual(items.map((x) => x.id).sort());
  });
  it("uses injected rng deterministically", () => {
    const seq = [0.1, 0.9, 0.3];
    let i = 0;
    const rng = () => seq[i++ % seq.length];
    const a = buildDeck(items.slice(0, 3), 3, rng);
    i = 0;
    const b = buildDeck(items.slice(0, 3), 3, rng);
    expect(a.map((x) => x.id)).toEqual(b.map((x) => x.id));
  });
});
