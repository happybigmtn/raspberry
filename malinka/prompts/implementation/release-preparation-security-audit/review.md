# Security Audit Lane — Review

Review only the current slice for `release-preparation-security-audit`.

Current Slice Contract:
Plan file:
- `genesis/plans/016-release-preparation.md`

Child work item: `release-preparation-security-audit`

Full plan context (read this for domain knowledge, design decisions, and specifications):

# Release Preparation

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This document must be maintained in accordance with [genesis/PLANS.md](/home/r/coding/fabro/genesis/PLANS.md).

## Purpose / Big Picture

After this change, Fabro/Raspberry is ready for a public release candidate. The README is accurate, CI passes on every PR, security audit findings are addressed, CHANGELOG exists, and the install path works. A technical user can install, run genesis, and start autodev without hitting known blockers.

The proof is: a fresh machine (Docker container with Rust toolchain) can install Fabro, run `fabro synth genesis` on a test repo, and `raspberry autodev` dispatches lanes successfully.

## Progress

- [ ] Final README and CHANGELOG review
- [ ] CI hardening: all checks pass on main
- [ ] Security audit of identified risks
- [ ] Fresh install test on clean machine
- [ ] Release binary build and test
- [ ] Version bump and tag

## Surprises & Discoveries

(To be updated)

## Decision Log

- Decision: Release as 0.x.0 (pre-1.0) to signal active development.
  Rationale: The system works for a single operator but has not been validated by external users. A 0.x release sets expectations correctly.
  Date/Author: 2026-03-26 / Genesis

- Failure scenario: The install script (`curl -fsSL https://fabro.sh/install.sh | bash`) may not work for all platforms or may download a stale binary. Mitigation: test install on Linux x86_64 and macOS ARM, and ensure install.sh pulls from the latest release tag.
  Date/Author: 2026-03-26 / Genesis

## Outcomes & Retrospective

(To be filled)

## Context and Orientation

Current release infrastructure:
- `.github/workflows/release.yml` — release workflow exists
- `install.md` and `install.sh` — install scripts exist at repo root
- `lib/crates/fabro-cli/build.rs` — embeds git SHA and build date in CLI version
- `CONTRIBUTING.md` — contribution model documented
- `LICENSE.md` — MIT license

Missing for release:
- No CHANGELOG
- CI doesn't enforce clippy
- Security findings from assessment not all addressed
- Install path untested on fresh machines
- No Docker-based CI test

## Milestones

### Milestone 1: README and CHANGELOG

Finalize `README.md` (from plan 014). Write `CHANGELOG.md` with the current release notes covering all genesis plan work.

Proof command:

    test -f CHANGELOG.md && head -20 CHANGELOG.md

### Milestone 2: CI hardening

Ensure `.github/workflows/rust.yml` includes:
- `cargo fmt --check --all`
- `cargo clippy --workspace -- -D warnings`
- `cargo nextest run --workspace`
- `cargo build --release -p fabro-cli -p raspberry-cli`

All checks must pass on the current `main` branch.

Proof command:

    cargo fmt --check --all && \
    cargo clippy --workspace -- -D warnings && \
    cargo nextest run --workspace && \
    cargo build --release -p fabro-cli -p raspberry-cli --target-dir target-local

### Milestone 3: Security audit

Address security findings from `genesis/ASSESSMENT.md`:
- Verify `shell_quote()` is used consistently for all shell interpolation
- Verify `--dangerously-skip-permissions` is documented and opt-in
- Verify `.env` files are in `.gitignore`
- Review direct trunk integration for safety on shared repos
- Check for any hardcoded credentials

Proof command:

    grep -rn "shell_quote\|shlex" lib/crates/fabro-workflows/src/ lib/crates/fabro-exe/src/ | wc -l
    grep ".env" .gitignore

### Milestone 4: Fresh install test

Create a Dockerfile that:
1. Starts from `rust:latest`
2. Installs Fabro from source
3. Creates a minimal test repo with SPEC.md and plans/
4. Runs `fabro synth genesis`
5. Runs `raspberry autodev --max-cycles 5`
6. Verifies at least one lane dispatches

Proof command:

    docker build -t fabro-install-test -f genesis/Dockerfile.test . && \
    docker run --rm fabro-install-test

### Milestone 5: Release binary build

Build release binaries for:
- Linux x86_64
- macOS ARM (aarch64-apple-darwin)

Verify the binaries run correctly on their target platforms.

Proof command:

    cargo build --release -p fabro-cli -p raspberry-cli --target-dir target-local && \
    target-local/release/fabro --version && \
    target-local/release/raspberry --version

### Milestone 6: Version bump and tag

Bump version in workspace `Cargo.toml`, create a git tag, and verify the release workflow runs.

Proof command:

    grep "version" Cargo.toml | head -1

## Validation and Acceptance

The plan is done when:
- README is accurate and CHANGELOG exists
- CI passes all checks on main
- Security findings are addressed or documented as accepted risk
- Fresh install works in Docker
- Release binaries build for Linux and macOS
- Version is tagged and release workflow succeeds


Workflow archetype: implement

Review profile: standard

Active plan:
- `genesis/plans/001-master-plan.md`

Active spec:
- `genesis/SPEC.md`

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

Review stage ownership:
- you may write or replace `.fabro-work/promotion.md` in this stage
- read `.fabro-work/quality.md` before deciding `merge_ready`
- when the slice is security-sensitive, perform a Nemesis-style pass: first-principles assumption challenge plus coupled-state consistency review
- include security findings in the review verdict when the slice touches trust boundaries, keys, funds, auth, control-plane behavior, or external process control
- prefer not to modify source code here unless a tiny correction is required to make the review judgment truthful
