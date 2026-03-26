# Audit Genesis Implementation Lane — Review

Review only the current slice for `genesis-onboarding-flow-audit-genesis-implementation`.

Current Slice Contract:
Plan file:
- `genesis/plans/012-genesis-onboarding-flow.md`

Child work item: `genesis-onboarding-flow-audit-genesis-implementation`

Full plan context (read this for domain knowledge, design decisions, and specifications):

# Genesis Onboarding Flow

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This document must be maintained in accordance with [genesis/PLANS.md](/home/r/coding/fabro/genesis/PLANS.md).

## Purpose / Big Picture

After this change, a new operator can go from an unfamiliar repo to autonomous code delivery in three commands:

    fabro synth genesis --target-repo /path/to/repo
    raspberry autodev --manifest /path/to/repo/malinka/programs/*.yaml --max-cycles 50
    raspberry tui --manifest /path/to/repo/malinka/programs/*.yaml

`fabro synth genesis` explores the repo as an interim CEO, writes SPEC.md + PLANS.md + numbered plans if they don't exist, then runs `synth create` to generate the full execution package. The second operator doesn't need to understand Fabro internals — but the binaries they run must expose the right commands, and the generated package must survive the handoff from repo checkout to detached run execution without local-only fixes.

The proof is: a repo that the original developer has never seen (not rXMRbro, not tonofcrap) goes from zero to supervised execution with genesis. The generated package has well-decomposed plans, correct dependencies, and working proof commands.

## Progress

- [x] Audit current `fabro synth genesis` implementation
- [x] Generate the genesis corpus on Fabro itself and inspect the outputs
- [ ] Make sure the CLI surface operators run actually exposes `fabro synth`
- [ ] Fix genesis/create runtime handoff so generated workflows validate without local prompt shims
- [ ] Add validation step after genesis that confirms package health
- [ ] Test on an unfamiliar open-source repo
- [ ] Write operator quickstart documentation

## Surprises & Discoveries

(To be updated)

## Decision Log

- Decision: `fabro synth genesis` should detect whether a repo already has plans and adapt accordingly.
  Rationale: Some repos will have SPEC.md and plans/ already. Genesis should import and synthesize from those. Bare repos need genesis to write the planning docs first.
  Date/Author: 2026-03-26 / Genesis

- Decision: Keep full genesis broad by default, but make the generated output self-auditing rather than blocking on a confirmation prompt.
  Rationale: The operator explicitly wants the broad scope preserved. The right fix is not "pause before create" — it is "make the generated report and validation output good enough that a second operator can decide what to do next without hidden knowledge."
  Date/Author: 2026-03-26 / Genesis

- Failure scenario: Genesis generates plans for a Python repo but picks Rust proof commands. Mitigation: genesis must detect the repo's primary language from file extensions and package manifests, then use language-appropriate proof commands.
  Date/Author: 2026-03-26 / Genesis

- Failure scenario: genesis succeeds in writing plans but the operator's binary does not expose `fabro synth`, or the generated package fails later because prompt refs only work from the original checkout. Mitigation: the onboarding path must include command-surface parity and detached-run validation as first-class acceptance criteria.
  Date/Author: 2026-03-26 / Genesis

## Outcomes & Retrospective

(To be filled)

## Context and Orientation

`fabro synth genesis` already exists (documented in `plans/032126-eng-review-brief.md`). It is described as: "For unfamiliar codebases: Opus explores as interim CEO, writes SPEC.md + PLANS.md + numbered plans, then runs synth create."

The command is implemented in `lib/crates/fabro-cli/src/commands/synth.rs`. The current implementation uses Claude Opus to explore the repo and generate planning documents. On 2026-03-26, genesis was run successfully on Fabro itself and produced `ASSESSMENT.md`, `SPEC.md`, `PLANS.md`, `DESIGN.md`, `GENESIS-REPORT.md`, and 16 numbered plans. The remaining issue is not "can genesis write the corpus?" It is "does the operator-facing CLI and the generated package execute cleanly after that?"

The onboarding flow end-to-end:

```
New operator with a repo
     |
     v
fabro synth genesis --target-repo /path/to/repo
     |
     ├──> Opus explores: build files, source structure, docs, git history
     ├──> Opus writes: SPEC.md, PLANS.md, plans/001-*.md through plans/N-*.md
     ├──> genesis report + validation output
     ├──> fabro synth create: blueprint → package
     └──> Output: malinka/ directory with programs, workflows, prompts
     |
     v
raspberry autodev --manifest malinka/programs/*.yaml
     |
     v
raspberry tui --manifest malinka/programs/*.yaml
```

## Milestones

### Milestone 1: Audit genesis implementation

Read `lib/crates/fabro-cli/src/commands/synth.rs` to understand the current genesis flow. Document what works, what's missing, and what's broken.

Proof command:

    grep -n "genesis" lib/crates/fabro-cli/src/commands/synth.rs | head -20

### Milestone 2: Repo detection and adaptation

Enhance genesis to detect:
- Primary language (Rust/TypeScript/Python/Go) from file extensions and package files
- Existing planning docs (SPEC.md, plans/, specs/)
- Build system (Cargo, npm/bun, pip, go)
- Test framework (nextest, vitest, pytest, go test)

When existing plans are found, import them instead of generating new ones.

Proof command:

    cargo nextest run -p fabro-cli -- synth genesis detection

### Milestone 3: Command-surface and runtime validation

After genesis generates plans and the execution package, verify:
- the operator-facing `fabro` binary exposes the `synth` subcommands that autodev will invoke later
- generated workflows can validate from the detached run environment
- prompt/workflow refs do not depend on a manually maintained global symlink

Proof command:

    fabro synth --help && \
    fabro validate /path/to/repo/malinka/run-configs/<family>/<lane>.toml

### Milestone 4: Validation step

After genesis generates plans and the execution package, run a validation step:
- Verify all plans have proof commands
- Verify proof commands match the detected language
- Verify the manifest references all generated lanes
- Report any warnings (orphaned lanes, circular dependencies)

Proof command:

    cargo nextest run -p fabro-synthesis -- genesis validation

### Milestone 5: Test on unfamiliar repo

Run genesis on an open-source Rust repo that the developer has never configured for Fabro. Verify:
- SPEC.md and plans/ are generated (if not present)
- synth create produces a valid package
- `raspberry plan-matrix` shows all plans as mapped
- `raspberry autodev --max-cycles 5` dispatches at least one lane

Proof command:

    # Manual test on a fresh repo
    fabro synth genesis --target-repo /tmp/test-repo
    raspberry plan-matrix --manifest /tmp/test-repo/malinka/programs/*.yaml

### Milestone 6: Operator quickstart documentation

Write a quickstart guide that covers:
1. Install Fabro
2. Run genesis on your repo
3. Review generated plans
4. Start autodev
5. Monitor with TUI

Write to `docs/guides/quickstart.mdx`.

Proof command:

    test -f docs/guides/quickstart.mdx && wc -l docs/guides/quickstart.mdx

Expected: >50 lines.

## Validation and Acceptance

The plan is done when:
- `fabro synth genesis` works on a repo with no existing Fabro config
- Genesis detects language and adapts proof commands
- Validation catches bad proof commands before autodev runs
- An unfamiliar repo goes from zero to dispatching lanes in 3 commands
- Quickstart documentation exists


Workflow archetype: implement

Review profile: standard

Active plan:
- `genesis/plans/001-master-plan.md`

Active spec:
- `genesis/SPEC.md`

Proof commands:
- `cargo nextest run -p fabro-cli -- synth genesis detection`
- `cargo nextest run -p fabro-synthesis -- genesis validation`

Artifacts to write:
- `implementation.md`
- `verification.md`
- `quality.md`
- `promotion.md`
- `integration.md`


Verification artifact must cover
- summarize the automated proof commands that ran and their outcomes

Nemesis-style security review
- Pass 1 — first-principles challenge: question trust boundaries, authority assumptions, and who can trigger the slice's dangerous actions
- Pass 2 — coupled-state review: identify paired state or protocol surfaces and check that every mutation path keeps them consistent or explains the asymmetry
- check secret handling, capability scoping, pairing/idempotence behavior, and privilege escalation paths

Focus on:
- slice scope discipline
- proof-gate coverage for the active slice
- touched-surface containment
- implementation and verification artifact quality
- remaining blockers before the next slice


Structural discipline
- if a new source file would exceed roughly 400 lines, split it before landing
- do not mix state transitions, input handling, rendering, and animation in one new file unless the prompt explicitly justifies that coupling
- if the slice cannot stay small, stop and update the artifacts to explain the next decomposition boundary instead of silently landing a monolith
Deterministic evidence:
- treat `.fabro-work/quality.md` as machine-generated truth about placeholder debt, warning debt, manual follow-up, and artifact mismatch risk
- if `.fabro-work/quality.md` says `quality_ready: no`, do not bless the slice as merge-ready


Score each dimension 0-10 and write `.fabro-work/promotion.md` in this exact form:

merge_ready: yes|no
manual_proof_pending: yes|no
completeness: <0-10>
correctness: <0-10>
convention: <0-10>
test_quality: <0-10>
reason: <one sentence>
next_action: <one sentence>

Scoring guide:
- completeness: 10=all deliverables present + all acceptance criteria met, 7=core present + 1-2 gaps, 4=missing deliverables, 0=skeleton
- correctness: 10=compiles + tests pass + edges handled, 7=tests pass + minor gaps, 4=some failures, 0=broken
- convention: 10=matches all project patterns, 7=minor deviations, 4=multiple violations, 0=ignores conventions
- test_quality: 10=tests import subject + verify all criteria, 7=most criteria tested, 4=structural only, 0=no tests

If `.fabro-work/contract.md` exists, verify EVERY acceptance criterion from it.
Any dimension below 6 = merge_ready: no.
If `.fabro-work/quality.md` says quality_ready: no = merge_ready: no.

For security-sensitive slices, append these mandatory fields exactly:
- layout_invariants_complete: yes|no
- slice_decomposition_respected: yes|no
If any mandatory security field is `no`, set `merge_ready: no`.

Review stage ownership:
- you may write or replace `.fabro-work/promotion.md` in this stage
- read `.fabro-work/quality.md` before deciding `merge_ready`
- when the slice is security-sensitive, perform a Nemesis-style pass: first-principles assumption challenge plus coupled-state consistency review
- include security findings in the review verdict when the slice touches trust boundaries, keys, funds, auth, control-plane behavior, or external process control
- prefer not to modify source code here unless a tiny correction is required to make the review judgment truthful
