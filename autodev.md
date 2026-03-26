# Autodev Restart Guide

This file is the shortest path to a correct 5-agent autodev restart.

## Contract

For child work:

- write/implement stages use the normal child workflow
- in-band adversarial review stays on `kimi-k2.5`
- integration/landing happens before any Codex pass

For post-child review:

- `*-plan-review` is the Kimi-driven in-band fixer
- `*-codex-review` is the separate after-completion Codex pass
- do not expect a running lane that started before regeneration to adopt new routing

If the live run violates that contract, rebuild, regenerate, and restart.

## Build

From `fabro/`:

```bash
cargo build --release -p fabro-cli -p raspberry-cli --target-dir target-local
```

Use these binaries for rollout:

- `target-local/release/fabro`
- `target-local/release/raspberry`

## Regenerate

Regenerate from the checked-in source blueprints, not from stale live state.

`rXMRbro`

```bash
target-local/release/fabro --no-upgrade-check synth create \
  --target-repo /home/r/coding/rXMRbro \
  --program rxmragent \
  --blueprint /home/r/coding/rXMRbro/malinka/blueprints/rxmragent.yaml \
  --no-decompose \
  --no-review
```

`tonofcrap`

```bash
target-local/release/fabro --no-upgrade-check synth create \
  --target-repo /home/r/coding/tonofcrap \
  --program repo \
  --blueprint /home/r/coding/tonofcrap/malinka/blueprints/repo.yaml \
  --no-decompose \
  --no-review
```

## Verify Routing

Before launch, verify both halves of the contract.

Child plan review should be Kimi:

```bash
sed -n '1,40p' /home/r/coding/rXMRbro/malinka/workflows/implementation/dice-plan-review.fabro
sed -n '1,40p' /home/r/coding/tonofcrap/malinka/workflows/implementation/premium-tables-withdrawal-plan-review.fabro
```

You should see:

- `#challenge   { ... model: kimi-k2.5; provider: kimi; }`
- `#review      { ... model: kimi-k2.5; provider: kimi; }`

Post-completion Codex review should be separate:

```bash
sed -n '1,40p' /home/r/coding/rXMRbro/malinka/workflows/recurring_report/dice-codex-review.fabro
sed -n '1,40p' /home/r/coding/tonofcrap/malinka/workflows/recurring_report/premium-tables-withdrawal-codex-review.fabro 2>/dev/null || true
```

You should see:

- `#review { ... model: gpt-5.4; provider: openai; }`

Manifest dependencies should show Codex only after plan review:

```bash
rg -n "codex-review|plan-reviewed|codex-reviewed" \
  /home/r/coding/rXMRbro/malinka/programs/rxmragent.yaml \
  /home/r/coding/tonofcrap/malinka/programs/repo.yaml
```

## Launch With 5 Agents

`rXMRbro`

```bash
target-local/release/raspberry autodev \
  --manifest /home/r/coding/rXMRbro/malinka/programs/rxmragent.yaml \
  --fabro-bin /home/r/coding/fabro/target-local/release/fabro \
  --max-parallel 5 \
  --max-cycles 1000000 \
  --poll-interval-ms 500 \
  --evolve-every-seconds 1800
```

`tonofcrap`

```bash
target-local/release/raspberry autodev \
  --manifest /home/r/coding/tonofcrap/malinka/programs/repo.yaml \
  --fabro-bin /home/r/coding/fabro/target-local/release/fabro \
  --max-parallel 5 \
  --max-cycles 1000000 \
  --poll-interval-ms 500 \
  --evolve-every-seconds 1800
```

## Monitor

Quick state:

```bash
sed -n '1,60p' /home/r/coding/rXMRbro/.raspberry/rxmragent-autodev.json
sed -n '1,60p' /home/r/coding/tonofcrap/.raspberry/repo-autodev.json
```

Expected healthy signals:

- `stop_reason: "InProgress"`
- `max_parallel: 5`
- `running: 5`
- no repeated invalid-lane errors in the live controller output

Inspect a live lane:

```bash
jq -r '.lanes["dice-plan-review:dice-plan-review"]' /home/r/coding/rXMRbro/.raspberry/rxmragent-state.json
```

Check which provider actually served a node:

```bash
sed -n '1,120p' /home/r/.fabro/runs/<run-id>/nodes/review/provider_used.json
sed -n '1,120p' /home/r/.fabro/runs/<run-id>/nodes/challenge/provider_used.json
```

Interpretation:

- child `plan-review` should show `provider: "kimi"`
- post-completion `codex-review` should show `provider: "openai"`

## Common Fixes

If a live run still shows Codex inside `*-plan-review`:

1. Stop the controller.
2. Rebuild `fabro`.
3. Regenerate from the source blueprint.
4. Relaunch.

If a repo has no `origin` remote:

- current Fabro should no-op trunk sync quietly
- if you still see repeated `git fetch origin failed`, rebuild and restart the controller

If a lane is emitted as `kind: integration` but uses an implementation workflow:

- fix the source blueprint or plan mapping
- regenerate the package
- restart the controller

Known example:

- `/home/r/coding/tonofcrap/malinka/plan-mappings/013-craps-school.yaml`

## One-Line Principle

After restart, child lanes do the real work with Minimax/Kimi first; Codex enters only in a separate `*-codex-review` lane after the child workflow is fully complete.
