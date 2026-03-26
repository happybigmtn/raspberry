# Fabro / Raspberry Design System

Status: Proposal
Date: 2026-03-26

## Context

Fabro has two user-facing surfaces:

1. **`raspberry tui`** — Terminal UI for program observation (ratatui + crossterm)
2. **`apps/fabro-web`** — React 19 web dashboard for run management, sessions, models

The TUI is Raspberry-native and actively used. The web UI is upstream Fabro and disconnected from Raspberry supervision. This design document covers both surfaces and proposes unification.

## Aesthetic Direction

**Industrial control room.** The operator is monitoring autonomous agents executing across multiple repos. The aesthetic should communicate: trustworthy, data-dense, status-at-a-glance. Think Bloomberg terminal meets GitHub Actions, not consumer SaaS.

Rationale: The primary user is a technical operator managing multiple autonomous pipelines. They need information density, not marketing polish. Every pixel should convey state.

## Typography

| Role | Font | Size | Weight |
|------|------|------|--------|
| Display / headings | JetBrains Mono | 1.5rem / 1.25rem | 700 |
| Body / descriptions | Inter | 1rem | 400 |
| UI labels / badges | Inter | 0.8125rem | 500 |
| Code / paths / commands | JetBrains Mono | 0.875rem | 400 |
| TUI | Terminal default (monospace) | — | — |

Modular scale: 1.25 (major third). Base: 16px.
Scale: 10.24 / 12.8 / 16 / 20 / 25 / 31.25 / 39.06

## Color Palette

### Primary

| Name | Hex | Usage |
|------|-----|-------|
| Raspberry | `#C72C48` | Primary brand, active elements, CTA |
| Raspberry Light | `#E8506A` | Hover states, active borders |
| Raspberry Dark | `#9B1B34` | Pressed states, headings |

### Semantic (status colors — the most important palette)

| Name | Hex | Usage |
|------|-----|-------|
| Complete | `#22C55E` | Lanes landed, milestones done |
| Running | `#3B82F6` | Active execution, in-progress |
| Ready | `#A855F7` | Dispatchable, awaiting slot |
| Blocked | `#F59E0B` | Dependency not met |
| Failed | `#EF4444` | Errors, crashes, review rejections |
| Surface Blocked | `#F97316` | External blocker (quota, permissions) |
| Idle | `#6B7280` | No work, settled |

### Neutrals (dark theme primary)

| Name | Hex | Usage |
|------|-----|-------|
| BG Primary | `#0F1117` | Main background |
| BG Secondary | `#1A1D27` | Cards, panels |
| BG Tertiary | `#252830` | Hover, selected rows |
| Border | `#2E3138` | Dividers, card borders |
| Text Primary | `#E5E7EB` | Body text |
| Text Secondary | `#9CA3AF` | Labels, descriptions |
| Text Muted | `#6B7280` | Timestamps, metadata |

### TUI Colors (256-color terminal safe)

| Name | ANSI | Usage |
|------|------|-------|
| Complete | Green (2) | Lane complete |
| Running | Blue (4) | Active lane |
| Ready | Magenta (5) | Ready to dispatch |
| Blocked | Yellow (3) | Blocked lane |
| Failed | Red (1) | Failed lane |
| Muted | DarkGray (8) | Metadata, timestamps |

## Spacing Scale

Base unit: 4px

| Token | Value | Usage |
|-------|-------|-------|
| `xs` | 4px | Inline spacing, badge padding |
| `sm` | 8px | Component internal padding |
| `md` | 16px | Card padding, between-element spacing |
| `lg` | 24px | Section spacing |
| `xl` | 32px | Page-level margins |
| `2xl` | 48px | Major section breaks |

## Layout: Web Dashboard

### Viewport: Desktop (>1200px)

```
+------------------------------------------------------------------+
|  [logo]  Programs  Runs  Models  Settings           [user] [?]   |
+------------------------------------------------------------------+
|          |                                                        |
|  Program |  Plan Matrix                                          |
|  List    |  +--------------------------------------------------+ |
|          |  | Plan     | Status  | Lanes | Landed | Next Move  | |
|  [repo1] |  | 001-foo  | 5/8     | 12    | 8      | dispatch   | |
|  [repo2] |  | 002-bar  | 3/5     | 7     | 3      | blocked    | |
|          |  | 003-baz  | done    | 4     | 4      | review     | |
|          |  +--------------------------------------------------+ |
|          |                                                        |
|          |  Selected Plan Detail                                  |
|          |  +--------------------------------------------------+ |
|          |  | Lane         | Status   | Model   | Duration     | |
|          |  | foo-core     | complete | minimax | 4m 32s       | |
|          |  | foo-tests    | running  | minimax | 2m 10s...    | |
|          |  | foo-review   | blocked  | kimi    | —            | |
|          |  +--------------------------------------------------+ |
+------------------------------------------------------------------+
```

### Viewport: Tablet (768-1200px)

Program list collapses to top selector dropdown. Plan matrix and lane detail stack vertically.

### Viewport: Mobile (<768px)

Not a priority surface. Show program status summary with drill-down to plan list. No lane-level detail on mobile.

## Layout: TUI

The existing `raspberry tui` layout is a three-pane dashboard:

```
+---------------------+------------------------------------------+
| Programs/Units      | Lane Detail                              |
| (grouped by status) |                                          |
|                     | Name: wallet-integration                 |
| [complete] 7        | Status: running                          |
| [running]  3        | Run ID: abc-123                          |
| [blocked]  188      | Stage: implement (2/5)                   |
| [failed]   26       | Duration: 4m 32s                         |
|                     | Last Output: "Writing wallet module..."  |
|                     |                                          |
|                     | Artifacts:                               |
| > mining-ops        |   spec.md ✓                              |
|   wallet-rpc ◀      |   review.md (pending)                    |
|   provably-fair     |   quality.md (pending)                   |
|   casino-core       |                                          |
+---------------------+------------------------------------------+
| Status bar: Cycle 42 | Dispatched: 3/5 | Budget: 2 remaining   |
+---------------------+------------------------------------------+
```

Navigation: `j/k` move, `Enter` expand, `q` quit, `Tab` switch panes.

## Interaction States

Every data-displaying component must handle these five states:

| State | Behavior |
|-------|----------|
| **Loading** | Skeleton with pulsing animation (web) or "Loading..." (TUI) |
| **Empty** | "No programs configured. Run `fabro synth create` to get started." |
| **Error** | Red banner with error message and retry button/command |
| **Partial** | Show available data with "Some data unavailable" indicator |
| **Success** | Full data display |

## Accessibility

- All interactive elements have `aria-label` attributes
- Color is never the sole indicator of state (icons/text accompany status colors)
- Focus ring visible on keyboard navigation (2px solid Raspberry, 2px offset)
- Minimum touch target: 44px x 44px
- Minimum contrast ratio: 4.5:1 for body text, 3:1 for large text
- TUI: all actions have keyboard shortcuts displayed in status bar

## Edge Cases

| Scenario | Handling |
|----------|----------|
| 47-character plan name | Truncate with ellipsis at 40 chars, full name on hover/focus |
| 0 programs | Empty state with setup instructions |
| 500+ lanes in one program | Virtualized list (web), paginated (TUI) |
| Network failure mid-refresh | Show last-known state with "Last updated: X" badge |
| First-time user | Guided setup flow pointing to `fabro synth genesis` |
| Stale `running` lane | Show duration with amber warning after 15min threshold |
