Goal: Autodev Execution Path and Dispatch Truth

Bootstrap the first honest reviewed slice for this frontier.

Inputs:
- `README.md`
- `SPEC.md`
- `PLANS.md`
- `AGENTS.md`
- `CLAUDE.md`
- `genesis/plans/001-master-plan.md`

Current frontier tasks:
- Eliminate local-only command and prompt-resolution shims from the autodev runtime path
- Fix stale `running` and `failed` lane truth before dispatch
- Add dispatch-state telemetry that explains why ready work did or did not run
- Live validation: sustain 10 active lanes on rXMRbro without bootstrap validation failures
- Live validation: at least 3 lanes land to trunk after the runtime path is boring

Required durable artifacts:
- `outputs/autodev-efficiency-and-dispatch/spec.md`
- `outputs/autodev-efficiency-and-dispatch/review.md`


## Completed stages
- **specify**: success
  - Model: MiniMax-M2.7-highspeed, 109.7k tokens in / 672 out
  - Files: outputs/autodev-efficiency-and-dispatch/review.md, outputs/autodev-efficiency-and-dispatch/spec.md
- **review**: success
  - Model: k2p5, 11.4k tokens in / 2.1k out


# Autodev Execution Path and Dispatch Truth Lane — Polish

Polish the durable artifacts for `autodev-efficiency-and-dispatch` so they are clear, repo-specific, and ready for the supervisory plane.
