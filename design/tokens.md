# Design tokens — Harus

All tokens are declared as CSS variables on `:root` in `Harus Quiz Platform.html`. Alternate themes override the same names under `[data-theme="paper"]` and `[data-theme="cobalt"]`. When porting to Tailwind, expose them in `tailwind.config.ts` `theme.extend.colors`.

## Color — Slate (default, dark)

| Token | Value | Use |
|---|---|---|
| `--bg`            | `#0e1116` | Page background |
| `--surface`       | `#161b22` | Cards, sidebar |
| `--surface-2`     | `#1d232c` | Elevated rows, code blocks, input wells |
| `--surface-3`     | `#232a35` | Deepest cards, avatar circle |
| `--border`        | `#262d38` | All standard borders, dashed KV dividers |
| `--border-strong` | `#353e4c` | Hover state for cards, strong dividers |
| `--text`          | `#e8ecf1` | Primary text |
| `--text-2`        | `#b4bdc9` | Secondary text |
| `--muted`         | `#7c8696` | Captions, mono kickers, axis labels |
| `--accent`        | `#22d3a8` | Primary CTA, active nav, correct outcomes, primary chart line |
| `--accent-dim`    | `rgba(34,211,168,0.12)` | Selected backgrounds, success callouts |
| `--accent-line`   | `rgba(34,211,168,0.35)` | Selected borders, success outlines |
| `--amber`         | `#f5b85a` | Warning, pending, "needs review" |
| `--amber-dim`     | `rgba(245,184,90,0.14)` | Warning callout backgrounds |
| `--red`           | `#e76e6e` | Error, wrong, revoke |
| `--red-dim`       | `rgba(231,110,110,0.14)` | Error callout backgrounds |
| `--blue`          | `#6aa7f5` | Informational, secondary chart line |
| `--blue-dim`      | `rgba(106,167,245,0.14)` | Informational backgrounds |

## Color — Paper (light academic)

| Token | Value |
|---|---|
| `--bg`            | `#f6f3ec` |
| `--surface`       | `#fbf9f3` |
| `--surface-2`     | `#f1ecdf` |
| `--surface-3`     | `#e7e0cc` |
| `--border`        | `#d9d1bc` |
| `--border-strong` | `#b9ad8e` |
| `--text`          | `#1d1a14` |
| `--text-2`        | `#3a352a` |
| `--muted`         | `#6e6753` |
| `--accent`        | `#0f6b53` |
| `--amber`         | `#a05c12` |
| `--red`           | `#a83b3b` |
| `--blue`          | `#2c5b8f` |

## Color — Cobalt (deep blue + gold)

| Token | Value |
|---|---|
| `--bg`            | `#0b1530` |
| `--surface`       | `#11203f` |
| `--surface-2`     | `#182a52` |
| `--surface-3`     | `#213665` |
| `--border`        | `#243868` |
| `--border-strong` | `#355088` |
| `--text`          | `#eaf0ff` |
| `--text-2`        | `#b6c4e6` |
| `--muted`         | `#7c8cb4` |
| `--accent`        | `#f5c14a` |
| `--amber`         | `#f08a3c` |
| `--red`           | `#ef6e6e` |
| `--blue`          | `#8db4ff` |

## Typography

| Token | Value |
|---|---|
| `--serif` | `"Source Serif 4", "Iowan Old Style", Georgia, serif` |
| `--sans`  | `"Inter", -apple-system, BlinkMacSystemFont, sans-serif` |
| `--mono`  | `"JetBrains Mono", "SF Mono", Menlo, monospace` |

### Type scale (used in design)

| Role | Family | Size | Weight | Tracking | Notes |
|---|---|---|---|---|---|
| Hero (signup) | serif | 56 | 500 | -1.2 | line-height 1.04 |
| Screen title  | serif | 26-30 | 500 | -0.3 to -0.4 | |
| Big score / stat | serif | 22-38 | 500 | -0.3 to -0.6 | |
| Card / section title | serif | 17-22 | 500 | -0.2 | |
| Item prompt | serif | 26 | 500 | -0.2 | line-height 1.35 |
| Body | sans | 13-14 | 400 | 0 | line-height 1.55-1.65 |
| UI label | sans | 13 | 500 | 0 | |
| Button | sans | 12-13 | 500 | 0.1 | sm = 12, md = 13 |
| Tag | sans | 11 | 500 | 0.3 | uppercase |
| Mono kicker | mono | 10-11 | 500 | 1.2-1.4 | uppercase |
| Mono metadata | mono | 10.5-12 | 400-500 | 0.3-0.8 | |
| Code block | mono | 12-13.5 | 400 | 0 | line-height 1.6 |

## Spacing

A loose 4/2 scale — `4 6 8 10 12 14 16 18 20 22 24 28 32 36 40 44 56`. No strict 8pt grid; use what reads best.

| Context | Value |
|---|---|
| Tightest inset (chips) | 5px / 10px |
| Field padding | 9-10 px / 12-14 px |
| Button padding | 6-11 px / 10-18 px (by size) |
| Card padding | 20-28 px |
| Page horizontal padding | 28-40 px |
| Section gap | 16-22 px |
| Item gap inside card | 6-14 px |

## Radii

| Token | Use |
|---|---|
| `4px` | Small chips, tag pills, code-block file rows |
| `6px` | Buttons, inputs, cards, badges, palette buttons |
| `10px` (`--radius-lg`) | Large hero cards |
| `999px` | Round category pills, toggle switch track |

## Borders

- Default: `1px solid var(--border)`
- Hover (interactive card): `1px solid var(--border-strong)`
- Active/selected: `1px solid var(--accent-line)` + `--accent-dim` background
- KV row dividers: `1px dashed var(--border)`
- Placeholder: `1px dashed var(--border-strong)`

## Component visual tokens

### Button

```text
primary  → bg: accent, text: #0b1410, border: accent
solid    → bg: surface-2, text: text, border: border
ghost    → bg: transparent, text: text-2, border: border
danger   → bg: transparent, text: red, border: border
quiet    → bg: transparent, text: muted, border: transparent
```

Sizes:

- sm: padding 6/10, font 12
- md: padding 8/14, font 13
- lg: padding 11/18, font 13

### Tag

```text
default → bg: surface-2, fg: text-2, border: border
accent  → bg: accent-dim, fg: accent, border: accent-line
amber   → bg: amber-dim, fg: amber, border: amber
red     → bg: red-dim, fg: red, border: red
blue    → bg: blue-dim, fg: blue, border: blue
ghost   → bg: transparent, fg: muted, border: border
```

All tags: padding 2/7, font 11, uppercase, letter-spacing 0.3.

### Input / textarea

- Background `var(--surface)`
- Border 1px `var(--border)`
- Border-radius 6px
- Padding 10/12 default; 16/18 for active-quiz inputs
- Font sans 13.5 (form fields), mono 12-13.5 (code), serif 16-18 (essay)
- No focus ring color shift in the mocks; in production, add `:focus-visible` outline using `var(--accent)`.

### Chart conventions

- Grid lines: `var(--border)` dashed `2 4`
- Axis labels: mono 10, `var(--muted)`, letter-spacing 0.4
- Line: `var(--accent)`, 1.8px, with a fill gradient from `rgba(accent,0.18)` to transparent
- Bar (filled): `var(--accent)` at 0.85 alpha on a `var(--surface-2)` track
- Histogram: bars `var(--surface-3)` with `var(--border)` outline; highlighted bin filled `var(--accent)`
- Scatter point: 6px circle, `var(--accent)` stroke + `rgba(accent,0.18)` fill
- Donut: 10px stroke, `var(--surface-2)` track, `var(--accent)` progress, rounded line cap

### Iconography

- Stroke width 1.6, line cap/join round, default size 16
- All inline SVG, see `Icon` export in `src/atoms.jsx`
- Available names: library, take, results, dashboard, author, agent, exam, stack, settings, arrow, arrowL, check, x, plus, clock, search, filter, bell, key, copy, download, upload, flag, user, sparkle, book, code

### Logo

22px default. Two strokes + horizontal bar, accent-colored rounded square, "Harus" wordmark in serif 600, letter-spacing -0.3.

## Motion

| Where | Duration | Easing | Property |
|---|---|---|---|
| Sidebar item state | 120ms | (default) | background, color |
| Button hover | 120ms | (default) | background, border-color, color |
| Card hover | 140ms | (default) | border-color, transform |
| Toggle knob | 140ms | (default) | left |
| Progress bar | 200ms | (default) | width |
| Theme switch | (instant) | — | variable swap |

No spring physics, no scroll-triggered reveals, no parallax. Restraint is part of the brand.

## Accessibility checklist

- Contrast: all `var(--text)` / `var(--bg)` pairings exceed WCAG AA (≥ 4.5:1). Verify on Paper theme specifically since the contrast is tightest there.
- All interactive elements are real `<button>` or `<a>` elements. When porting, keep that.
- Add visible `:focus-visible` outlines (the mocks omit them for tidiness) — recommend `outline: 2px solid var(--accent); outline-offset: 2px;`
- Question palette buttons should announce their state with `aria-current` (current question) and `aria-label` ("Question 4 — answered, flagged").
- Timer should be `role="timer" aria-live="off"` until the last 5 minutes, then switch to `aria-live="polite"` to announce.
- Code editor: provide a real accessible label and keep `spellCheck` off.
