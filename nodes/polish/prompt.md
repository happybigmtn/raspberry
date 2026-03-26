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
- **review**: success
  - Model: k2p5, 2.3k tokens in / 433 out
  - Files: outputs/provider-policy-stabilization/review.md


# Provider Policy Stabilization Lane — Polish

Polish the durable artifacts for `provider-policy-stabilization` so they are clear, repo-specific, and ready for the supervisory plane.
