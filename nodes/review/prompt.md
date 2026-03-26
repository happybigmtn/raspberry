Goal: Greenfield Bootstrap and Runtime Asset Reliability

Bootstrap the first honest reviewed slice for this frontier.

Inputs:
- `README.md`
- `SPEC.md`
- `PLANS.md`
- `AGENTS.md`
- `CLAUDE.md`
- `genesis/plans/001-master-plan.md`

Current frontier tasks:
- Verify scaffold-first ordering works in planning.rs
- Add bootstrap verification gate to render.rs
- Make generated prompt and workflow asset refs runtime-stable
- Add type-aware quality checks for TypeScript projects
- Validate on tonofcrap with 30-cycle autodev run
- Validate on a fresh Rust project with scaffold dependency

Required durable artifacts:
- `outputs/greenfield-bootstrap-reliability/spec.md`
- `outputs/greenfield-bootstrap-reliability/review.md`


## Completed stages
- **specify**: success
  - Model: MiniMax-M2.7-highspeed, 82.5k tokens in / 472 out


# Greenfield Bootstrap and Runtime Asset Reliability Lane — Review

Review the lane outcome for `greenfield-bootstrap-reliability`.

Focus on:
- correctness
- milestone fit
- remaining blockers


Nemesis-style security review
- Pass 1 — first-principles challenge: question trust boundaries, authority assumptions, and who can trigger the slice's dangerous actions
- Pass 2 — coupled-state review: identify paired state or protocol surfaces and check that every mutation path keeps them consistent or explains the asymmetry
- check secret handling, capability scoping, pairing/idempotence behavior, and privilege escalation paths
- check external-process control, operator safety, idempotent retries, and failure modes around service lifecycle