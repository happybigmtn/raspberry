Goal: Raspberry Supervisor Edge Case Tests

Child work item of plan: Test Coverage for Critical Paths

Proof commands:
- `cargo nextest run -p raspberry-supervisor -- integration autodev_cycle`
- `cargo nextest run -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed`

Required durable artifacts:
- `implementation.md`
- `verification.md`
- `quality.md`
- `promotion.md`
- `integration.md`


## Completed stages
- **preflight**: success
  - Script: `set +e
if cargo nextest --version >/dev/null 2>&1; then
  cargo nextest run -p raspberry-supervisor -- integration autodev_cycle && cargo nextest run -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed
else
  cargo test -p raspberry-supervisor -- integration autodev_cycle && cargo test -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed
fi
true`
  - Stdout: (empty)
  - Stderr:
    ```
    (16 lines omitted)
            PASS [   0.005s] (4/5) raspberry-supervisor autodev::tests::replayable_failed_lanes_replay_source_lane_for_failed_integration_program
            FAIL [   0.505s] (5/5) raspberry-supervisor integration::tests::integrate_lane_squash_merges_run_branch_into_trunk
      stdout ───
    
        running 1 test
        test integration::tests::integrate_lane_squash_merges_run_branch_into_trunk ... FAILED
    
        failures:
    
        failures:
            integration::tests::integrate_lane_squash_merges_run_branch_into_trunk
    
        test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 118 filtered out; finished in 0.50s
    
      stderr ───
    
        thread 'integration::tests::integrate_lane_squash_merges_run_branch_into_trunk' (3409727) panicked at lib/crates/raspberry-supervisor/src/integration.rs:268:10:
        integration succeeds: Direct(Git { step: "resolve ssh push url", repo: "/home/r/.cache/rust-tmp/fabro-direct-integration-29bb3659-50a0-476e-a9dc-544753f9b49a", message: "Engine error: remote `origin` must use SSH or be convertible from GitHub HTTPS, got `/home/r/.cache/rust-tmp/.tmphG3RbA/remote.git`" })
        note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
    
      Cancelling due to test failure: 
    ────────────
         Summary [   0.505s] 5 tests run: 4 passed, 1 failed, 114 skipped
            FAIL [   0.505s] (5/5) raspberry-supervisor integration::tests::integrate_lane_squash_merges_run_branch_into_trunk
    error: test run failed
    ```
- **contract**: success
  - Model: MiniMax-M2.7-highspeed, 44.5k tokens in / 230 out
- **implement**: success
  - Model: MiniMax-M2.7-highspeed, 128.0k tokens in / 506 out
  - Files: docs/rpi-edge-case-tests.md, lib/crates/raspberry-supervisor/src/integration.rs, test/fixtures/raspberry-supervisor/.raspberry/myosu-program-state.json, test/fixtures/raspberry-supervisor/.raspberry/program-state.json
- **verify**: fail
  - Script: `if cargo nextest --version >/dev/null 2>&1; then
  cargo nextest run -p raspberry-supervisor -- integration autodev_cycle && cargo nextest run -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed
else
  cargo test -p raspberry-supervisor -- integration autodev_cycle && cargo test -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed
fi`
  - Stdout: (empty)
  - Stderr:
    ```
    (49 lines omitted)
        [autodev-step] program=parent cycle=1 program-before-evaluated
        [autodev-step] program=parent cycle=1 frontier ready=0 running=0 replayable=0 regenerable=0 doctrine_changed=false
        [autodev-step] program=parent cycle=1 running-synth-evolve
        [autodev] synth evolve failed for program `parent`: fabro synth evolve for program `parent` failed with exit_status=1; continuing
        [autodev-step] program=parent cycle=1 evolve-skipped
        [autodev-step] program=parent cycle=1 program-after-evolve-evaluated
        [autodev-step] program=parent cycle=1 dispatch-plan available_slots=1 replayed=0 ready=0 dispatching=0
        [autodev-step] program=parent cycle=1 dispatch-complete outcomes=0
        [autodev-step] program=parent cycle=1 program-after-dispatch-skipped-no-changes
        [autodev-step] program=parent cycle=1 report-saved
    
        thread 'autodev::tests::orchestrate_program_reports_recursive_child_program_cycles' (3776939) panicked at lib/crates/raspberry-supervisor/src/autodev.rs:4563:10:
        recursive program cycle should fail cleanly: AutodevReport { program: "parent", stop_reason: Settled, updated_at: 2026-03-26T23:04:38.693430354Z, provenance: Some(AutodevProvenance { controller: BinaryProvenance { path: "/home/r/coding/fabro/.raspberry/cargo-target/debug/deps/raspberry_supervisor-a7b858c092b4bdea", version: None }, fabro_bin: BinaryProvenance { path: "/bin/false", version: None } }), current: Some(AutodevCurrentSnapshot { updated_at: 2026-03-26T23:04:38.693428970Z, max_parallel: Some(1), ready: 0, running: 0, blocked: 1, failed: 0, complete: 0, ready_lanes: [], running_lanes: [], failed_lanes: [], critical_blockers: [] }), cycles: [AutodevCycleReport { cycle: 1, evolved: false, evolve_target: None, ready_lanes: [], replayed_lanes: [], regenerate_noop_lanes: [], dispatched: [], running_after: 0, complete_after: 0 }] }
        note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
    
      Cancelling due to test failure: 5 tests still running
            PASS [   0.063s] (20/24) raspberry-supervisor autodev::tests::ensure_target_repo_fresh_for_dispatch_blocks_dirty_repo_that_is_behind
            PASS [   0.064s] (21/24) raspberry-supervisor autodev::tests::ensure_target_repo_fresh_for_dispatch_fast_forwards_with_only_untracked_noise
            PASS [   0.064s] (22/24) raspberry-supervisor autodev::tests::ensure_target_repo_fresh_for_dispatch_fast_forwards_clean_default_branch
            PASS [   0.835s] (23/24) raspberry-supervisor program_state::tests::refresh_program_state_marks_missing_running_run_as_stale_failure
            PASS [   0.906s] (24/24) raspberry-supervisor program_state::tests::refresh_program_state_clears_stale_failure_residue_for_succeeded_run_progress
    ────────────
         Summary [   0.906s] 24 tests run: 23 passed, 1 failed, 95 skipped
            FAIL [   0.013s] (19/24) raspberry-supervisor autodev::tests::orchestrate_program_reports_recursive_child_program_cycles
    error: test run failed
    ```

## Context
- failure_class: canceled
- failure_signature: verify|canceled|script failed with exit code: <n> ## stderr finished `test` profile [unoptimized + debuginfo] target(s) in <n>.18s ──────────── nextest run id <hex>-60cd-4dc8-ba1d-<hex> with nextest profile: default starting <n> tes


# Raspberry Supervisor Edge Case Tests Lane — Fixup

Fix only the current slice for `test-coverage-critical-paths-raspberry-supervisor-edge-case-tests`.


Verification artifact must cover
- summarize the automated proof commands that ran and their outcomes

Priorities:
- unblock the active slice's first proof gate — this is the #1 priority
- prefer staying within the named slice and touched surfaces
- if the proof gate fails on pre-existing issues OUTSIDE your surfaces (e.g., linter warnings in unrelated files, missing imports in dependencies), you MUST fix those issues minimally to unblock the gate — do not leave the lane stuck on problems you can solve
- preserve setup constraints before expanding implementation scope
- keep implementation and verification artifacts durable and specific
- do not create or rewrite `.fabro-work/promotion.md` during Fixup; that file is owned by the Review stage
- do not hand-author `.fabro-work/quality.md`; the Quality Gate rewrites it after verification
- ALL ephemeral files (quality.md, promotion.md, verification.md) go in `.fabro-work/`, never the repo root
