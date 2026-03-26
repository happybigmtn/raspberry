# Fabro / Raspberry Genesis Assessment

Date: 2026-03-26
Assessor: Genesis (interim CEO/CTO review)

## The ONE Sentence

Fabro is an AI-powered workflow orchestration engine that compiles planning documents into executable, multi-agent pipelines — now extending into Raspberry, a repo-level supervisory plane that autonomously schedules, monitors, and lands AI-generated code across entire codebases.

---

## Six Forcing Questions

### 1. Demand Reality

**Who uses this?** The primary user is the repo owner (solo founder/operator, `happygrandmtn@gmail.com`). Three proving-ground repos — `rXMRbro` (Rust casino), `tonofcrap` (TypeScript Telegram Mini App), and `Zend`/`Myosu` — are being actively supervised by Raspberry autodev. The autodev loop has dispatched 580+ cycles across 221 lanes in rXMRbro alone, landing real code to trunk autonomously.

**What proves demand?** The overnight autodev runs burning real LLM credits ($2.50+/megatoken MiniMax, OpenAI Codex quota exhausted mid-run) prove the operator values this enough to spend money and compute on it. The 21 plans written in 8 days (March 18-26) and 50+ commits in the same period prove obsessive iteration velocity. But there are no external users, no payments from others, no public adoption signal.

**Verdict:** This is a high-conviction single-operator project with real autonomous execution capability, but zero external demand signal yet. It's past prototype stage — real code lands on trunk — but pre-product-market-fit.

### 2. Status Quo

**Without Fabro/Raspberry, the workflow is:** manually prompt AI agents per-file, manually review, manually integrate. The operator would use Claude Code, Codex, or similar tools one conversation at a time, with no repo-level supervision, no automatic retry/recovery, no multi-lane parallelism, and no checkpoint/resume.

**Duct-tape alternatives:**
- Claude Code + manual prompting (works but no orchestration)
- OpenAI Codex (single-shot, no supervision loop)
- Devin/Factory/Cursor (commercial alternatives, but no plan-first discipline)
- GitHub Actions + custom scripts (possible but enormous manual effort)

**Pain it solves:** The gap between "AI can write code" and "AI can autonomously deliver a multi-crate project against a planning corpus." Nobody else ships plan-first autonomous execution with adversarial review, automatic retry, and direct trunk integration.

### 3. Desperate Specificity

**The ONE person:** A solo technical founder building multiple Rust/TypeScript projects simultaneously. They have strong opinions about code quality (adversarial review, sprint contracts, scored evaluation). They stay up at night worrying that AI-generated code is landing without sufficient verification. They get promoted (or their startup succeeds) when their repos ship autonomously — multiple projects progressing in parallel without their constant attention.

**Title:** Solo technical founder / independent software operator
**Constraint:** Time — one person, multiple repos, AI does the coding but someone needs to supervise the supervisors
**What keeps them up:** "Did the autodev loop land broken code to trunk?" "Are 83% of cycles really idle with ready work?"

### 4. Narrowest Wedge

**What someone would pay for THIS WEEK:**

`fabro synth genesis` — point it at any repo with planning docs (SPEC.md, plans/) and get a fully decomposed, supervised execution package. Then `raspberry autodev` runs the whole thing with automatic retry, adversarial review, and trunk integration. This is the kernel: **turn a planning corpus into autonomous code delivery.**

The wedge is NOT the workflow engine (upstream Fabro). The wedge is the synthesis + supervision + autodev loop that turns plans into landed code without human intervention.

### 5. Observation & Surprise

**Surprise 1: The codebase is enormous for a solo project.** 206,345 lines of Rust across 28 crates, 5,036 commits. The operator has been building at an extraordinary pace — 21 plans in 8 days. This is either superhuman productivity or AI-assisted development of AI-assisted development tools (the system is building itself).

**Surprise 2: 2,952 `unwrap()` calls in production code.** For a project that emphasizes code quality and adversarial review, the core engine itself has significant error handling debt. Only 8 TODO/FIXME comments (suspiciously clean — either the code is well-maintained or TODOs are aggressively deleted).

**Surprise 3: The autodev efficiency is poor.** From the 032326 plan: 580 cycles, 16.2% dispatch rate, 83.3% idle with ready work, 0 lanes landed to main. The system is burning cycles without landing code. The operator knows this and is iterating fast, but the core loop has fundamental efficiency problems.

**Surprise 4: The plans/ directory is the real product.** The 21 ExecPlan files represent the highest-quality artifact in the repo. They follow a rigorous living-document discipline (progress checkboxes with timestamps, decision logs, surprises & discoveries). The plans are better-maintained than the code they describe.

**Surprise 5: Model routing chaos.** The provider policy has changed repeatedly — gpt-5.4 removed for quota, MiniMax primary, Kimi K2.5 for review, Opus for synthesis, mutual failover everywhere. The `fabro-model/src/policy.rs` centralization was a recent fix for leaking hardcoded model choices.

**Surprise 6: Half-built Paperclip integration.** The Paperclip dashboard sync exists but the web UI (`apps/fabro-web`) appears disconnected from the Raspberry control plane. The web app has its own auth, setup flow, and API — it's the upstream Fabro web UI, not a Raspberry-native dashboard.

**Surprise 7: The `fabro-cli/src/main.rs` has uncommitted changes** visible in git status. The operator is actively modifying the CLI.

### 6. Future-Fit

**How it compounds:** As AI models improve, the autodev loop gets more effective without code changes — better implementations, better reviews, fewer retry cycles. As more repos adopt plan-first development, the synthesis engine has more training signal. The adversarial review + sprint contract pattern is a durable competitive advantage — it's not just "run AI", it's "run AI with quality discipline."

**How it could decay:** If frontier models gain native multi-file orchestration (Claude Projects, Codex autonomous mode), the workflow engine layer becomes less valuable. If commercial alternatives (Devin, Factory) ship plan-first execution, the differentiation narrows. The biggest decay risk is complexity — 28 crates and 206K LOC for a solo project means maintenance burden grows faster than feature velocity.

**The bet:** AI agents need supervision, not just execution. The bet is that plan-first, adversarially-reviewed autonomous development is the right abstraction — and that the open-source workflow engine wins against closed commercial alternatives.

---

## What Works

1. **Plan-first discipline** — `PLANS.md` and `SPEC.md` governance is well-designed and consistently followed
2. **Synthesis pipeline** — `fabro synth create/evolve` reliably decomposes plans into executable packages
3. **Autodev loop** — `raspberry autodev` orchestrates multi-lane execution with retry, recovery, and scheduling
4. **Direct trunk integration** — settled lanes land on main without PRs when configured
5. **Adversarial review stack** — 3-tier review (Minimax → Opus → Codex), sprint contracts, scored evaluation
6. **Plan registry** — deterministic plan → workflow mapping via `raspberry-supervisor/src/plan_registry.rs`
7. **TUI observer** — `raspberry tui` provides real-time program observation
8. **Paperclip sync** — frontier state exports to Paperclip dashboard with issue/document sync

## What's Broken

1. **Autodev efficiency** — 83% idle cycles, 16% dispatch rate, 0 trunk landings in 580-cycle run
2. **`unwrap()` epidemic** — 2,952 production unwrap() calls across the engine that supervises other code for quality
3. **Greenfield bootstrap** — parallel lane dispatch before project scaffold exists (tonofcrap failure)
4. **Provider routing instability** — repeated model policy changes, quota exhaustion, hardcoded fallbacks
5. **Stale runtime state** — `running` lanes that are actually dead, resource lease staleness
6. **Web UI disconnect** — `apps/fabro-web` is upstream Fabro web, not Raspberry-native

## What's Half-Built

1. **Plan completion detection** — autodev doesn't know when all lanes of a plan are done
2. **Protocol contract verification** — `BlueprintProtocol` struct proposed but not wired
3. **Workspace-level integration testing** — per-crate tests pass, workspace fails
4. **Portfolio scheduler** — module exists in tests, not wired to dispatch
5. **Shadow-mode plan-centric cutover** — lane-centric → plan-centric dispatch transition
6. **First-class PlanReview workflow** — reuses generic implementation template instead
7. **Resource lease liveness** — lease tracking exists but doesn't validate daemon health
8. **Harness simplification assessment** — Phase 4 of sprint contracts plan (A/B test challenge stage)

---

## Tech Debt Inventory

| Area | Issue | File(s) | Severity |
|------|-------|---------|----------|
| Error handling | 2,952 `unwrap()` calls in production | All crates, esp. `fabro-workflows` (52K LOC) | High |
| Error handling | 15 `unsafe` blocks | Various | Medium |
| Testing | `fabro-db` has zero tests | `lib/crates/fabro-db/` | High |
| Testing | `fabro-types` has zero tests | `lib/crates/fabro-types/` | Low (auto-generated) |
| Testing | No integration/E2E test CI for Raspberry | `.github/workflows/rust.yml` | High |
| Architecture | `fabro-workflows` is 52K LOC monolith | `lib/crates/fabro-workflows/src/` | Medium |
| Architecture | `fabro-cli` is 32K LOC | `lib/crates/fabro-cli/src/` | Medium |
| Provider policy | Hardcoded model choices leak outside `policy.rs` | `render.rs`, `cli.rs`, `synth.rs` | Medium |
| Web UI | `fabro-web` disconnected from Raspberry | `apps/fabro-web/` | High |
| Documentation | No user-facing docs for Raspberry commands | `docs/` | Medium |
| Build | No `cargo clippy` enforcement in CI | `.github/workflows/rust.yml` | Medium |

## Security Risks

| Risk | Location | Severity |
|------|----------|----------|
| Shell command injection via `shell_quote()` | `fabro-workflows/src/`, `fabro-exe/src/` | Medium — mitigated by `shlex::try_quote` but surface area is large |
| `--dangerously-skip-permissions` in synth | `fabro-cli/src/commands/synth.rs` | High — synthesis runs Claude with no permission gates |
| `git reset --hard` in autodev (recently removed) | `raspberry-supervisor/src/autodev.rs` | Low — removed, but pattern may recur |
| Ambient credentials for LLM providers | `.env` file, environment variables | Medium — standard pattern but no secret rotation |
| npm global install races | `fabro-workflows/src/backend/cli.rs` | Low — recently fixed with file lock |
| Direct trunk integration without human review | `fabro-workflows/src/direct_integration.rs` | Medium — by design, but risky for shared repos |

## Test Coverage Gaps

| Crate | LOC | Tests | Coverage Assessment |
|-------|-----|-------|---------------------|
| `fabro-db` | ~1,500 | 0 | **No tests at all** — SQLite + WAL mode + migrations untested |
| `fabro-types` | ~5,000 | 0 | Auto-generated, low risk |
| `fabro-synthesis` | 15,472 | 88 | Synthesis pipeline critical, moderate coverage |
| `raspberry-supervisor` | 15,049 | 114 | Core control plane, needs more edge case tests |
| `fabro-workflows` | 52,263 | 831 | Largest crate, but 831 tests for 52K LOC is thin |
| `fabro-mcp` | ~1,800 | 7 | MCP client barely tested |
| `fabro-github` | ~1,200 | 4 | GitHub integration barely tested |
| `fabro-beastie` | unknown | unknown | New crate, unknown coverage |

---

## Existing Plan Assessment

### Strong Plans (carry forward as-is or with minor enhancement)

| Plan | Rating | Genesis Action |
|------|--------|----------------|
| `031826-bootstrap-raspberry-supervisory-plane` | **Strong** — completed, clean progress, good decision log | Carry forward as historical reference |
| `031826-port-and-generalize-fabro-dispatch-for-myosu` | **Strong** — completed, thorough generalization work | Carry forward as historical reference |
| `031926-build-skill-guided-program-synthesis` | **Strong** — completed, the synthesis pipeline is working | Carry forward as historical reference |
| `031926-extend-fabro-create-workflow-for-raspberry` | **Strong** — completed, skill-first authoring surface shipped | Carry forward as historical reference |
| `031926-implement-raspberry-run-observer-tui` | **Strong** — completed, TUI is working | Carry forward as historical reference |
| `032026-sync-paperclip-with-raspberry-frontiers` | **Strong** — completed, Paperclip sync working | Carry forward as historical reference |
| `032126-eng-review-brief` | **Strong** — excellent summary doc for plan-first architecture | Carry forward as reference |

### Plans Needing Enhancement

| Plan | Rating | Genesis Action |
|------|--------|----------------|
| `031926-build-raspberry-autodev-orchestrator` | **Good but needs efficiency work** — autodev runs but 83% idle | Enhance with efficiency milestones |
| `031926-harden-autonomy-and-direct-trunk-integration` | **Good but incomplete** — integration works, bootstrap needs polish | Enhance with greenfield hardening |
| `032026-keep-fabro-and-raspberry-continuously-generating-work` | **Good but incomplete** — frontier budgeting exists, continuous generation needs proof | Enhance with measured efficiency targets |
| `032026-harden-run-reliability-from-myosu-rxmrbro-zend` | **Good, mostly done** — direct integration hardened, stale state fixed | Carry forward, remaining items in new plan |
| `032126-plan-first-autodev-redesign` | **Strong design, execution partially done** — plan registry and synthesis wired, but portfolio scheduler and shadow cutover missing | Enhance with implementation milestones |
| `032526-parent-holistic-review-shipping-gauntlet` | **Strong, recently landed** — parent gauntlet synthesized, needs live validation | Carry forward with live proof milestone |

### Weak or Incomplete Plans (need significant work)

| Plan | Rating | Genesis Action |
|------|--------|----------------|
| `032326-autodev-efficiency-and-harness-engineering` | **Weak as plan, strong as post-mortem** — no ExecPlan structure, just metrics/analysis | Merge into efficiency-focused genesis plan |
| `032326-centralize-provider-policy-and-live-autodev-recovery` | **Partially done** — policy centralized, live recovery incomplete | Merge remaining items into reliability plan |
| `032426-greenfield-bootstrapping-and-code-quality` | **Weak** — problem statement only, no ExecPlan format, no progress tracking | Replace with structured genesis plan |
| `032426-harness-redesign-sprint-contracts-and-evaluation` | **Good proposal, partially implemented** — sprint contracts landed (6a89dc3c), scored review landed, but no A/B test or simplification assessment | Enhance with measurement milestones |
| `032426-integration-verification-and-codebase-polish` | **Good proposal, not implemented** — workspace verification and protocol contracts proposed | Carry forward into genesis plan |
| `032526-e2e-autodev-review-and-remediation` | **Mixed** — Phase 1 done, Phase 2-3 have open items | Remaining items into new plans |
| `032526-plan-level-adversarial-review-and-recursive-improvement` | **Good architecture, needs implementation** — plan-completion detection not wired | Carry forward into genesis plan |
| `032626-structural-remediation-from-landed-code-review` | **Strong, just landed** — settlement hygiene and invariant synthesis done | Carry forward with live rollout milestone |

### Contradictions and Overlaps

1. **Review routing conflict:** Multiple plans change the review model chain (032326-centralize-provider-policy, 032526-e2e, 032526-parent-holistic). The centralized policy should be the single source of truth.
2. **Plan-review duplication:** Three plans propose plan-level review improvements (032526-plan-level-adversarial, 032526-parent-holistic, 032526-e2e Phase 3). These should be one coherent plan.
3. **Greenfield overlap:** 032426-greenfield and 032426-harness-redesign both address quality failures in freshly bootstrapped projects, from different angles. Should be one plan.

---

## Architecture Snapshot

```
                            +-----------------+
                            |  plans/*.md     |
                            |  specs/*.md     |
                            +-------+---------+
                                    |
                                    v
                  +----------------------------------+
                  |  fabro-synthesis                  |
                  |  (planning.rs, render.rs,         |
                  |   blueprint.rs)                   |
                  +----------------------------------+
                          |                    |
                          v                    v
           +------------------+    +-------------------------+
           | malinka/programs/ |    | malinka/workflows/      |
           | *.yaml manifests  |    | malinka/run-configs/    |
           +--------+---------+    | malinka/prompts/        |
                    |              +------------+------------+
                    v                           |
           +------------------+                 |
           | raspberry-       |                 |
           | supervisor       |                 |
           | (autodev.rs,     |<----------------+
           |  dispatch.rs,    |
           |  evaluate.rs,    |
           |  plan_status.rs) |
           +--------+---------+
                    |
                    v
           +------------------+
           | fabro-workflows  |
           | (backend/cli.rs, |
           |  handler/*.rs,   |
           |  direct_         |
           |  integration.rs) |
           +--------+---------+
                    |
        +-----------+-----------+
        |           |           |
        v           v           v
   +---------+ +--------+ +---------+
   | fabro-  | | fabro- | | fabro-  |
   | agent   | | llm    | | sandbox |
   +---------+ +--------+ +---------+
                    |
        +-----------+-----------+
        |           |           |
        v           v           v
   Anthropic   OpenAI    MiniMax/Kimi
```

## Crate Dependency Graph (workspace-internal)

```
fabro-cli ──> fabro-workflows, fabro-agent, fabro-llm, fabro-api,
              fabro-synthesis, fabro-config, fabro-telemetry,
              fabro-validate, fabro-github, fabro-mcp, fabro-model

fabro-workflows ──> fabro-agent, fabro-llm, fabro-sandbox,
                    fabro-graphviz, fabro-hooks, fabro-config,
                    fabro-git-storage, fabro-tracker, fabro-util

fabro-synthesis ──> raspberry-supervisor, fabro-model

raspberry-supervisor ──> (standalone — no workspace deps)
raspberry-cli ──> raspberry-supervisor, raspberry-tui
raspberry-tui ──> raspberry-supervisor

fabro-agent ──> fabro-llm, fabro-sandbox, fabro-util
fabro-api ──> fabro-workflows, fabro-llm, fabro-db, fabro-types, fabro-config
fabro-llm ──> fabro-config, fabro-util
fabro-sandbox ──> (standalone)
```
