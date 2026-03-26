# 180-Day Turnaround Plan — Fabro / Raspberry

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This document must be maintained in accordance with [genesis/PLANS.md](/home/r/coding/fabro/genesis/PLANS.md).

## Purpose / Big Picture

After 180 days, Fabro/Raspberry should be a product that a second operator can run against their own repos without the original author's help. The system should autonomously execute a multi-plan program, land code to trunk, report progress via dashboard, and recover from common failures — all from `fabro synth genesis` to `raspberry autodev` with no manual glue.

The proof is not architecture diagrams. The proof is: a new operator clones a repo, writes planning docs, runs three commands, and watches code land on trunk.

The broad scope stays intact, but the execution order changes. The first gate is no longer "more features" — it is "generated packages run cleanly on real repos without local-only shims, missing CLI entrypoints, or broken prompt/workflow resolution."

## Progress

- [ ] Phase 0 kickoff: stabilization sprint
- [ ] Phase 1 kickoff: foundation hardening
- [ ] Phase 2 kickoff: growth features
- [ ] Phase 3 kickoff: polish and release

## Surprises & Discoveries

(To be updated as work proceeds)

## Decision Log

- Decision: Organize the 180-day plan around four phases with clear gates between them.
  Rationale: The codebase has active development velocity but significant stability gaps. Stabilization must come before growth features.
  Date/Author: 2026-03-26 / Genesis

- Decision: Carry forward all 21 existing plans from `plans/` into the genesis corpus, either as-is, enhanced, or explicitly dropped with rationale.
  Rationale: The existing plans represent significant domain knowledge and operator intent. Silently dropping them would lose institutional memory.
  Date/Author: 2026-03-26 / Genesis

- Decision: Limit to 15 numbered plans (002-016) to keep the portfolio manageable.
  Rationale: More than 15 plans for a solo operator creates scheduling overhead that exceeds the coordination benefit. Each plan must be independently executable.
  Date/Author: 2026-03-26 / Genesis

- Decision: Archived plans are hypotheses until revalidated on the live codebase.
  Rationale: Several archived plans were already partially completed before genesis ran. Their findings remain useful, but they should not control sequencing unless they still reproduce in the current repo and proving-ground runs.
  Date/Author: 2026-03-26 / Genesis

- Decision: Prioritize execution-path consistency before UI expansion.
  Rationale: The highest-leverage work is still in the path from `fabro synth genesis` / `fabro synth create` to `raspberry autodev` dispatching real lanes. Browser surfaces matter more after the generated package, runtime path resolution, and dispatch loop are boringly reliable.
  Date/Author: 2026-03-26 / Genesis

- Failure scenario: If Phase 0 takes longer than 30 days, the operator is likely fixing too many things at once. The mitigation is to ruthlessly cut scope — only fix what blocks autodev from landing code.
  Date/Author: 2026-03-26 / Genesis

## Outcomes & Retrospective

(To be filled at phase gates)

---

## Phase 0: Stabilization (Days 1-30)

**Goal:** The generated package and the autodev loop are boringly runnable on proving-ground repos. Generated workflows resolve their prompts and assets correctly, CLI entrypoints exist in the binaries operators actually run, and the controller can keep 10 real lanes active without manual rescue.

**Gate:** On a proving-ground repo, `raspberry autodev --max-parallel 10` sustains 10 running lanes for at least 20 cycles, produces zero bootstrap-time validation failures caused by missing CLI subcommands or unresolved prompt/workflow refs, and lands at least 3 lanes to trunk.

| # | Plan | Focus | Key Deliverable |
|---|------|-------|-----------------|
| 003 | Autodev Execution Path & Dispatch Truth | Fix runtime path resolution, stale state, and dispatch consistency | Proving-ground repo holds 10 active agents without local-only shims |
| 004 | Greenfield Bootstrap Reliability | Scaffold-first ordering, bootstrap verification, runtime asset resolution | Fresh repos bootstrap cleanly and generated workflows resolve their assets correctly |
| 005 | Test Coverage for Critical Paths | Regression tests for synthesis, dispatch, validation, and workspace state | CI catches package/runtime regressions before autodev does |
| 008 | Provider Policy Stabilization | Stable routing, quota-aware fallback, operator-visible provider health | Provider failure no longer collapses active frontier work |

## Phase 1: Foundation (Days 31-90)

**Goal:** The system validates itself at the repo level. Error handling, workspace verification, and genesis onboarding are trustworthy enough that a second operator can run the core loop without hidden wiring knowledge.

**Gate:** Critical-path error handling is hardened, workspace verification lanes run automatically for multi-lane plans, and `fabro synth genesis` can produce a runnable package on an unfamiliar repo without manual command surgery.

| # | Plan | Focus | Key Deliverable |
|---|------|-------|-----------------|
| 002 | Error Handling Hardening | Replace critical unwrap() calls with structured failures | Engine failures surface as actionable errors, not panics |
| 010 | Workspace Integration Verification | Auto-generate workspace-verify and protocol-contract lanes | Cross-crate failures are caught before parent review |
| 006 | Sprint Contracts & Scored Review | Contract stage, scoring rubric, measurable reject signals | Review quality is explainable and enforceable |
| 012 | Genesis Onboarding Flow | Three-command unfamiliar-repo onboarding | Second operator succeeds on a repo the original author did not pre-shape |

## Phase 2: Growth (Days 91-150)

**Goal:** Plan-level review, ship gates, and operator-facing visibility become first-class. The system stops thinking in isolated lanes and starts thinking in plan completion and release boundaries.

**Gate:** Parent gauntlet stages run live on real plans, plan completion automatically triggers review boundaries, and integration commits remain clean and attributable.

| # | Plan | Focus | Key Deliverable |
|---|------|-------|-----------------|
| 007 | Plan Completion & Adversarial Review | Wire plan-completion detection, 3-step adversarial review | Plans auto-trigger review on completion |
| 011 | Parent Review Gauntlet Rollout | Live validation of holistic preflight→retro pipeline | Parent plans ship through gauntlet |
| 013 | Settlement Hygiene & Evidence Separation | Integration commits contain only owned product files | Clean diffs on landed code |
| 009 | Web Dashboard Raspberry Integration | Connect fabro-web to Raspberry plan matrix and autodev state | Browser view reflects Raspberry truth without becoming the scheduler |

## Phase 3: Polish (Days 151-180)

**Goal:** The system is documented, performance is measured, and the release is ready for public announcement.

**Gate:** Documentation covers all operator commands. Performance baselines exist. README reflects current reality.

| # | Plan | Focus | Key Deliverable |
|---|------|-------|-----------------|
| 014 | Documentation & Operator Runbook | User-facing docs for all Raspberry commands | New operator self-serves |
| 015 | Performance Measurement & Optimization | Autodev cycle time, dispatch rate, trunk landing rate metrics | Dashboard shows operational health |
| 016 | Release Preparation | README, CHANGELOG, CI hardening, security audit | Public release candidate |

---

## Plan Dependency Graph

```
Phase 0 (broad scope, but strict ordering inside it):
  003-execution-path-truth ───────┐
  004-bootstrap-reliability ──────┤──> Phase 0 Gate
  005-critical-path-tests ────────┤
  008-provider-policy ────────────┘

Phase 1 (after Phase 0 gate):
  002-error-handling ─────────────┐
  010-workspace-verification ─────┤──> Phase 1 Gate
  006-sprint-contracts ───────────┤
  012-genesis-onboarding ─────────┘
       |
       +── 002 depends on 003/005 (stabilize runtime first, then harden failure paths)
       +── 010 depends on 005 (tests and fixtures first)
       +── 012 depends on 003/004/008 (onboarding cannot be better than the execution path it invokes)

Phase 2 (after Phase 1 gate):
  007-plan-completion ────────────┐
  011-parent-gauntlet ────────────┤──> Phase 2 Gate
  013-settlement-hygiene ─────────┤
  009-web-dashboard ──────────────┘
       |
       +── 007 depends on 006 and 010 (review boundaries only matter once contracts and workspace verification are real)
       +── 011 depends on 007 (plan completion triggers parent review)
       +── 009 depends on 003 and 011 (dashboard should expose proven runtime truth, not wishful state)

Phase 3 (after Phase 2 gate):
  014-documentation ──────────────┐
  015-performance ────────────────┤──> Phase 3 Gate (180-day mark)
  016-release ────────────────────┘
       |
       +── 016 depends on 014 and 015
```

## Existing Plan Mapping

Every plan from `plans/` is accounted for:

| Existing Plan | Genesis Plan | Action |
|---------------|-------------|--------|
| `031826-bootstrap-raspberry-supervisory-plane` | — | Completed. Historical reference. |
| `031826-port-and-generalize-fabro-dispatch-for-myosu` | — | Completed. Historical reference. |
| `031926-build-skill-guided-program-synthesis` | — | Completed. Historical reference. |
| `031926-extend-fabro-create-workflow-for-raspberry` | — | Completed. Historical reference. |
| `031926-implement-raspberry-run-observer-tui` | — | Completed. Historical reference. |
| `031926-build-raspberry-autodev-orchestrator` | 003 | Efficiency work continues |
| `031926-harden-autonomy-and-direct-trunk-integration` | 004, 013 | Split: bootstrap → 004, settlement → 013 |
| `032026-keep-fabro-and-raspberry-continuously-generating-work` | 003 | Merged: frontier budgeting → efficiency |
| `032026-sync-paperclip-with-raspberry-frontiers` | 009 | Enhanced: Paperclip → web dashboard |
| `032026-harden-run-reliability-from-myosu-rxmrbro-zend` | 003, 008 | Split: dispatch → 003, provider → 008 |
| `032126-eng-review-brief` | — | Reference document. No action needed. |
| `032126-plan-first-autodev-redesign` | 007 | Portfolio scheduler and shadow cutover → 007 |
| `032326-autodev-efficiency-and-harness-engineering` | 003 | Merged: post-mortem findings → efficiency |
| `032326-centralize-provider-policy-and-live-autodev-recovery` | 008 | Enhanced: remaining items → 008 |
| `032426-greenfield-bootstrapping-and-code-quality` | 004 | Replaced: structured genesis plan |
| `032426-harness-redesign-sprint-contracts-and-evaluation` | 006 | Enhanced: add measurement milestones |
| `032426-integration-verification-and-codebase-polish` | 010 | Carried forward with implementation plan |
| `032526-e2e-autodev-review-and-remediation` | 003, 007 | Split: controller truth → 003, review → 007 |
| `032526-plan-level-adversarial-review-and-recursive-improvement` | 007 | Merged: adversarial review architecture → 007 |
| `032526-parent-holistic-review-shipping-gauntlet` | 011 | Carried forward with live rollout |
| `032626-structural-remediation-from-landed-code-review` | 013 | Carried forward with settlement hygiene |
