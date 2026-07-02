# design/preview/ — static preview harness

Self-contained HTML mockups for the **current** `ame` (飴 × Hatsune Miku) visual
direction, served over the LAN/Tailscale so they can be checked on real devices
(phone, iPad).

> Distinct from the files in `design/` one level up — those are the **legacy**
> "Harus" academic-serif handoff (kept as historical reference). This folder is
> the live design exploration.

These are **mockups only** — the real theme lives in
`web/src/components/ThemeRegistry.tsx`. Nothing here is bundled or deployed.

## Serve

```bash
make preview                    # serves design/preview/ on 0.0.0.0:28900
make preview PREVIEW_PORT=8010  # override the port
```

Open from any Tailscale device:

- Gallery: `http://harus-mini:28900/`
- A board directly: `http://harus-mini:28900/styleboard.html`

## Add a board

Drop a self-contained `*.html` here (inline CSS/SVG, no external deps so it
renders offline) and add a card to `index.html`.

## Boards

- `index.html` — gallery
- `styleboard.html` — palette, components, the "Due today" review card, candy-jar streak, dark mode
- `elements.html` — Miku motif candidates beyond palette, each tagged by licensing risk
