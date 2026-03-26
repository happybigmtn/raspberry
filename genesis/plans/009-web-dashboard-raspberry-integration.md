# Web Dashboard Raspberry Integration

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This document must be maintained in accordance with [genesis/PLANS.md](/home/r/coding/fabro/genesis/PLANS.md).

## Purpose / Big Picture

After this change, an operator can open a browser and see the same Raspberry truth that the TUI and JSON reports already expose: plan matrix, lane status, dispatch rates, and review scores. The web dashboard should become a faithful companion surface, not a second scheduler and not a speculative detour from execution reliability.

The proof is: start `fabro serve` and open `http://localhost:8080`. The dashboard shows the plan matrix for a configured program. Lane status updates as autodev runs. Click a plan to see its child lanes, review scores, and artifacts.

Provenance: This plan enhances the Paperclip sync work from `plans/032026-sync-paperclip-with-raspberry-frontiers.md` and addresses the assessment finding that `apps/fabro-web` is disconnected from Raspberry.

## Progress

- [ ] Add Raspberry plan-matrix API endpoint to fabro-api
- [ ] Add autodev status API endpoint to fabro-api
- [ ] Build plan matrix component in fabro-web
- [ ] Build lane detail component with review scores
- [ ] Connect SSE event stream for live updates
- [ ] Design review against DESIGN.md

## Surprises & Discoveries

(To be updated)

## Decision Log

- Decision: Keep this plan behind the execution-path and verification work.
  Rationale: A browser surface is valuable, but it should reflect proven runtime truth. The repo should not spend Phase 0 effort on a richer UI while generated packages and proving-ground autodev still require local workarounds.
  Date/Author: 2026-03-26 / Genesis

- Decision: The web dashboard shows read-only Raspberry state in the first version. No dispatch/control actions from the browser.
  Rationale: Read-only is safe and useful. Write actions (dispatch, restart, cancel) require careful authentication and authorization that should be a separate plan.
  Date/Author: 2026-03-26 / Genesis

- Failure scenario: SSE event stream for live updates may not scale if autodev produces hundreds of state changes per second. Mitigation: debounce state updates to at most 1 per second, batch lane status changes.
  Date/Author: 2026-03-26 / Genesis

## Outcomes & Retrospective

(To be filled)

## Context and Orientation

The web frontend lives in `apps/fabro-web/` (React 19 + React Router + Vite + Tailwind). The API server lives in `lib/crates/fabro-api/` (Axum). The OpenAPI spec is at `docs/api-reference/fabro-api.yaml`.

The current web app has pages for: runs, sessions, models, completions. It does NOT have pages for: plan matrix, lane status, autodev state, review scores, dispatch rates.

The Raspberry state that needs to be surfaced:
- Plan matrix from `raspberry-supervisor/src/plan_status.rs`
- Lane status from `raspberry-supervisor/src/program_state.rs`
- Autodev report from `raspberry-supervisor/src/autodev.rs`
- Review scores from `.fabro-work/promotion.md` artifacts

```
+--------------------------------------------------+
|  fabro-api (Axum)                                |
|                                                  |
|  GET /api/raspberry/plans       → plan matrix    |
|  GET /api/raspberry/lanes/:id   → lane detail    |
|  GET /api/raspberry/autodev     → autodev report |
|  GET /api/raspberry/events      → SSE stream     |
+--------------------------------------------------+
         |
         v
+--------------------------------------------------+
|  fabro-web (React 19)                            |
|                                                  |
|  /raspberry          → plan matrix dashboard     |
|  /raspberry/plan/:id → plan detail + lanes       |
|  /raspberry/lane/:id → lane detail + artifacts   |
+--------------------------------------------------+
```

## Milestones

### Milestone 1: Plan-matrix API endpoint

Add `GET /api/raspberry/plans` to `lib/crates/fabro-api/src/server.rs` that returns the plan status matrix as JSON. Accept a `manifest` query parameter pointing to the program manifest file.

Key files:
- `lib/crates/fabro-api/src/server.rs` — new route handler
- `docs/api-reference/fabro-api.yaml` — OpenAPI spec extension

Proof command:

    cargo nextest run -p fabro-api -- raspberry plans
    # Also: curl http://localhost:8080/api/raspberry/plans?manifest=/path/to/manifest.yaml

### Milestone 2: Autodev status API endpoint

Add `GET /api/raspberry/autodev` that returns the current autodev report (cycle count, dispatch rate, active lanes, last cycle timestamp).

Proof command:

    cargo nextest run -p fabro-api -- raspberry autodev

### Milestone 3: Plan matrix React component

Build a `PlanMatrix` component in `apps/fabro-web/` that fetches from the plan-matrix API and renders a table with: Plan name, Status (with semantic color), Lane count, Landed count, Next move.

Follow `genesis/DESIGN.md` for layout, colors, and interaction states.

Proof command:

    cd apps/fabro-web && bun run typecheck && bun test

### Milestone 4: Lane detail component

Build a `LaneDetail` component that shows: lane status, model assignment, duration, review scores (if available), artifact links. Click a lane in the plan matrix to see this detail.

Proof command:

    cd apps/fabro-web && bun run typecheck && bun test

### Milestone 5: SSE live updates

Connect the existing SSE infrastructure in fabro-api to push Raspberry state changes. The plan matrix and lane detail components should update automatically when autodev changes state.

Key file: `lib/crates/fabro-api/src/server.rs` (SSE endpoint)

Proof command:

    cd apps/fabro-web && bun run build
    # Manual: open browser, run autodev, confirm live updates

### Milestone 6: Design review

Review the implemented components against `genesis/DESIGN.md` checklist:
- [ ] All 5 interaction states handled (loading, empty, error, partial, success)
- [ ] Semantic status colors match design spec
- [ ] Keyboard navigation works
- [ ] Responsive layout for desktop (>1200px) and tablet (768-1200px)
- [ ] Accessibility: focus rings, aria-labels, contrast ratio

Proof command:

    cd apps/fabro-web && bun run build && bun run typecheck

## Validation and Acceptance

The plan is done when:
- `fabro serve` serves Raspberry plan matrix at a browser URL
- Plan matrix shows live lane status with semantic colors
- Lane detail shows review scores and artifacts
- SSE stream updates the UI as autodev runs
- Design review passes against DESIGN.md checklist
