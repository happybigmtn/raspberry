## Objective

Repair the current `rXMRbro` baseline and add harness-level enforcement in
`fabro` so the same quality and throughput failures stop before they become
10-lane autodev debt.

## Scope

1. Repair immediate `rXMRbro` baseline blockers.
2. Tighten test helper semantics so contention is not misreported as missing
   state.
3. Add controller admission gating for wide autodev runs.
4. Extend synthesized quality gates with structural, money-domain, and
   interaction-performance checks.
5. Validate lane proof commands against owned surfaces before synthesis.

## Execution

### Phase 1: Baseline repair in `rXMRbro`

- Restore `bin/house` test support as a real `#[cfg(test)]` module.
- Remove current warning debt blocking zero-warning enforcement.
- Replace money-like float accounting in TUI screens with integral accounting.
- Remove the busy-loop skip path in Roulette.

### Phase 2: Harness semantics in `rXMRbro`

- Make blocking session helpers surface lock contention as `Busy`.
- Preserve "not found" semantics only for real absence.
- Update test call sites to use the new helper contracts.

### Phase 3: Admission gate in `fabro`

- Add a pre-dispatch workspace admission check for wide autodev runs.
- Require `cargo check --workspace` to pass with zero warnings.
- Fail fast with an actionable controller-level error.

### Phase 4: Synthesized quality gates in `fabro`

- Extend implementation quality checks with:
  - file-size budget debt
  - function-size budget debt
  - money-type debt for chip/session P&L state
  - interaction-performance debt for input-handler busy loops
- Keep evidence-only lanes exempt where appropriate.

### Phase 5: Blueprint validation in `fabro`

- Add lane-spec verification before synthesis.
- Reject obvious proof-command mismatches against owned surfaces.
- Block broad workspace proof commands where the lane already owns a narrower
  crate or file target.

## Verification

- `cargo check --workspace` in `rXMRbro`
- targeted `bin/house` tests for session helpers and commitment support
- targeted TUI screen tests for Red Dog, Sic Bo, and Roulette behavior
- `cargo test`/`cargo check` for touched `fabro` crates:
  - `fabro-synthesis`
  - `raspberry-supervisor`

## Exit Criteria

- `rXMRbro` has a clean workspace baseline with zero warnings for touched areas.
- `fabro` prevents wide autodev launch from a red or warning-producing repo.
- Synthesized lanes fail early on structural bloat, money-type drift, busy-loop
  UI paths, and obvious proof-command mismatch.
