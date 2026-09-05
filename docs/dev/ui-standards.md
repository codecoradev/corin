# Corin UI Standards

Codified during the v0.4 redesign (issue #299). Ported from uteke-mobile's
AGENTS.md discipline where noted. These rules are review gates, not suggestions.

## Color

- **Color lock:** teal is the ONLY brand action color (buttons, active nav,
  links, focus). Amber/yellow is semantic-only (match scores, warnings).
  Blue/mauve are for agent identity and data-viz — never primary actions.
- All colors via tokens (`src/app.css`, spec in `DESIGN.md`). No raw color
  literals in components.
- Text contrast: every text role must pass WCAG AA in **both** themes.

## Shape & motion

- **3-token radius:** `--radius-md` (10) cards/inputs, `--radius-lg` (14)
  modals/sheets, `--radius-pill` CTAs/chips. `--radius-sm` (5) only for tiny
  inline elements (kbd, code).
- Interactions: 150ms `--ease-out`; press scale 0.98 on tappable cards
  (PressScale pattern, from mobile).

## Type

- Sans for UI prose; **mono mandatory** for IDs, dates, counts, and anything
  tabular (`font-variant-numeric: tabular-nums` is global already).
- Reading measure ~65ch; relative timestamps in feeds ("2m ago").

## Components & states

- Empty states use the shared `EmptyState` (icon + title + subtitle + CTA).
  Never hand-rolled columns.
- Selected/active states need **2+ visual changes** (color + icon/weight/
  background) — from mobile P24.
- Every destructive action previews consequences before executing
  (ConfirmDialog with counts; dry-run where applicable).
- Raw UUIDs never shown inline — Copy-ID disclosure (first 8 chars + copy).
- One language per locale for chrome UI; agent content stays as authored.

## Layout

- Primary rail (global) + secondary panel (per view) + content; panel is
  collapsible; <768px becomes drawer.
- Horizontal overflow at any viewport >= 320px is a bug (deterministic gate
  in the QA script).

## QA gate

`scripts/qa_screens.py` must pass (0 console errors, 0 overflow) for any
UI-touching PR; screenshots in dark+light desktop and mobile are attached to
the PR and reviewed with the vision checklist (contrast, hierarchy, copy).
