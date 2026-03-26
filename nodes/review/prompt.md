Goal: Provider Policy Stabilization

Bootstrap the first honest reviewed slice for this frontier.

Inputs:
- `README.md`
- `SPEC.md`
- `PLANS.md`
- `AGENTS.md`
- `CLAUDE.md`
- `genesis/plans/001-master-plan.md`

Current frontier tasks:
- Audit all code paths that select models outside policy.rs
- Add quota detection and graceful fallback to fabro-llm providers
- Add provider health status to raspberry status output
- Add usage tracking per provider per cycle
- Live validation: quota exhaustion triggers fallback without lane failure

Required durable artifacts:
- `outputs/provider-policy-stabilization/spec.md`
- `outputs/provider-policy-stabilization/review.md`


## Completed stages
- **specify**: success
  - Model: MiniMax-M2.7-highspeed, 66.9k tokens in / 564 out
  - Files: outputs/provider-policy-stabilization/review.md, outputs/provider-policy-stabilization/spec.md


# Provider Policy Stabilization Lane — Review

Review the lane outcome for `provider-policy-stabilization`.

Focus on:
- correctness
- milestone fit
- remaining blockers


Nemesis-style security review
- Pass 1 — first-principles challenge: question trust boundaries, authority assumptions, and who can trigger the slice's dangerous actions
- Pass 2 — coupled-state review: identify paired state or protocol surfaces and check that every mutation path keeps them consistent or explains the asymmetry
- check state transitions that affect balances, commitments, randomness, payout safety, or replayability
- check secret handling, capability scoping, pairing/idempotence behavior, and privilege escalation paths
- check external-process control, operator safety, idempotent retries, and failure modes around service lifecycle