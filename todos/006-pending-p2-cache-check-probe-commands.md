---
status: pending
priority: p2
issue_id: "006"
tags: [raspberry, autodev, checks, performance]
dependencies: []
---

# Cache or throttle check probe command execution

Reduce repeated shell command probes during program evaluation so lane checks stop dominating the
poll loop.

## Problem Statement

Each command-based lane check spawns a new `bash -lc` process and can wait up to five seconds
before timing out. Because evaluation walks every lane serially, a program with many command probes
can burn a large fraction of each autodev cycle just re-running the same health checks.

## Findings

- Command probes are executed through `bash -lc` for every check evaluation:
  [evaluate.rs](/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/evaluate.rs#L1573)
- The timeout budget is five seconds per command:
  [evaluate.rs](/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/evaluate.rs#L29)
- The current implementation has no per-cycle memoization, probe TTL, or shared result cache.

## Proposed Solutions

### Option 1: Per-evaluation command result cache

**Approach:** Cache identical command probe results within one `evaluate_program()` call.

**Pros:**
- Low-risk and local change
- Helps immediately when multiple lanes reuse the same command

**Cons:**
- Does not help repeated polls across cycles

**Effort:** 1-2 hours

**Risk:** Low

---

### Option 2: Add probe TTLs to runtime state

**Approach:** Reuse recent successful probe results for a short window instead of executing on
every poll.

**Pros:**
- Large steady-state throughput improvement
- Reduces shell churn and timeout exposure

**Cons:**
- Makes health checks slightly less instantaneous
- Needs explicit freshness semantics

**Effort:** 3-5 hours

**Risk:** Medium

## Recommended Action

Implement Option 1 first, then consider a short TTL for expensive checks that do not need
sub-second freshness.

## Acceptance Criteria

- [ ] Duplicate command probes are not re-executed within a single evaluation pass
- [ ] Tests cover cache hits and timeout behavior
- [ ] Evaluation latency improves on programs with repeated command probes

## Work Log

### 2026-03-25 - Review Discovery

**By:** Codex

**Actions:**
- Traced command probe execution and timeout behavior
- Flagged the lack of memoization or freshness windows

**Learnings:**
- The current implementation is simple and reliable, but it scales poorly with repeated checks

