# Corin Design Tokens

> Source of truth for the v0.4 redesign (issues #291, epic #289).
> Decisions locked 2026-09-06: dark = editor-grade (default), light = warm paper.
> **Theme = palette-only** — typography, spacing, radius are identical across themes.
> Discipline (ported from uteke-mobile AGENTS.md): 3-token radius, color lock
> (teal = brand action, amber/yellow = semantic score/warning only), tabular
> figures for dynamic numbers, selected states need 2+ visual changes.

## Palettes

| Token | Dark (default) | Light | Role |
|---|---|---|---|
| `--color-base` | `#0E1015` | `#FBF8F1` | app background |
| `--color-mantle` | `#12151D` | `#FFFDF8` | cards, panels |
| `--color-crust` | `#171B26` | `#F3EEE3` | raised surfaces |
| `--color-surface0` | `#222836` | `#E7E0D2` | hairline borders + hover tint |
| `--color-surface1` | `#2A3242` | `#DDD5C6` | strong hover / active chrome |
| `--color-surface2` | `#3A4254` | `#CFC6B5` | pressed / deepest chrome |
| `--color-text` | `#E6E9EF` | `#2E2A1F` | primary text |
| `--color-subtext` | `#8B93A7` | `#6E6650` | secondary text |
| `--color-overlay` | `#5A6478` | `#A39A85` | muted text |
| `--color-teal` | `#4FD8D2` | `#0E7C74` | **brand accent** (all primary actions) |
| `--color-yellow` | `#E89B3C` | `#C07A3A` | semantic only: match scores, warnings |
| `--color-green` | `#34D399` | `#3E9B6C` | success / live status |
| `--color-red` | `#F87171` | `#C25B4E` | danger |
| `--color-blue` | `#7CB2FF` | `#5B7DB1` | agent identity A |
| `--color-mauve` | `#C4A7FF` | `#8B6FC7` | agent identity B |
| `--color-peach` | `#E0AF68` | `#B8742F` | reserved |

Legacy accent alias: `--accent` → teal. Blue stays available for identity/data-viz, never for primary actions.

## Radius (3 semantic values only)

| Token | Value | Use |
|---|---|---|
| `--radius-md` | 10px | cards, inputs |
| `--radius-lg` | 14px | modals, sheets |
| `--radius-pill` | 999px | CTAs, chips, badges |
| `--radius-sm` | 5px | tiny inline (kbd, code) |

## Type & motion

- Sans UI: system stack (kept); Mono: JetBrains Mono stack — mandatory for IDs, dates, counts (`font-variant-numeric: tabular-nums` is already global).
- Motion: 150ms `--ease-out`; press scale 0.98 on tappable cards (PressScale pattern from mobile).

## Semantic tint system

`--color-*-bg` (14%) / `--color-*-line` (30%) derive via `color-mix` from the base tokens — both themes get correct tints automatically. Never hardcode `rgba()`.

## Enforcement

- No raw color literals outside this file's token blocks (grep gate in review).
- Text contrast must pass WCAG AA in **both** themes (checked by QA journey script, #299).
