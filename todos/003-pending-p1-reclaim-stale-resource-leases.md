---
status: pending
priority: p1
issue_id: "003"
tags: [raspberry, autodev, resource-leases, reliability]
dependencies: []
---

# Reclaim stale autodev resource leases

Fix Raspberry's Zend daemon resource lease lifecycle so crashed runs cannot strand ports and block
future autodev dispatch.

## Problem Statement

`resource_lease.rs` persists Zend daemon port leases to disk, but the lease format has no owner
identity and stale leases are only removed when a later controller refresh sees that a lane key is
not currently running. If the controller or worker dies before that cleanup path runs, leases can
accumulate until the port range is exhausted and new runs fail with
`NoAvailableZendDaemonPort`.

## Findings

- `ResourceLease` stores only `lane_key`, `port`, and `acquired_at`, so the system cannot verify
  whether the original owner is still alive:
  [resource_lease.rs](/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/resource_lease.rs#L63)
- `cleanup_leases()` removes leases solely by comparing persisted `lane_key` values to the current
  running set, which does not help after controller crashes, renamed lanes, or stale state files:
  [resource_lease.rs](/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/resource_lease.rs#L137)
- `acquire_zend_daemon_lease()` never tries to reclaim stale files before giving up on the entire
  port range:
  [resource_lease.rs](/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/resource_lease.rs#L173)

## Proposed Solutions

### Option 1: Add process ownership to resource leases

**Approach:** Persist PID and process start ticks, then reclaim any lease whose owner is no longer
alive before trying the next port.

**Pros:**
- Matches the existing controller-lease safety model
- Fixes crash recovery without needing a healthy controller loop

**Cons:**
- Linux `/proc` based unless abstracted
- Requires a schema bump and migration tolerance

**Effort:** 2-4 hours

**Risk:** Medium

---

### Option 2: Add lease TTL plus periodic sweep

**Approach:** Expire resource leases that have not been refreshed within a bounded time window.

**Pros:**
- Portable across environments
- Simpler than PID validation

**Cons:**
- Time-based reclamation can steal a lease from a slow but healthy run
- Requires refresh logic from active workers

**Effort:** 3-5 hours

**Risk:** Medium

## Recommended Action

Implement Option 1 first. Reuse the controller lease ownership pattern, tolerate older lease files,
and add tests proving stale leases are reclaimed on startup before the allocator reports exhaustion.

## Technical Details

**Affected files:**
- [resource_lease.rs](/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/resource_lease.rs)
- [controller_lease.rs](/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/controller_lease.rs)

## Acceptance Criteria

- [ ] Resource leases persist enough ownership metadata to detect stale owners
- [ ] Lease acquisition reclaims dead-owner leases before reporting exhaustion
- [ ] Tests cover controller crash or abandoned lease recovery
- [ ] New runs no longer fail solely because of leftover lease files

## Work Log

### 2026-03-25 - Review Discovery

**By:** Codex

**Actions:**
- Reviewed Raspberry autodev lease handling end to end
- Traced Zend daemon lease allocation and cleanup paths
- Confirmed there is no stale-owner reclamation path in the allocator

**Learnings:**
- The controller lease already has a stronger stale-owner model that can be reused here
- The current resource lease cleanup assumes the controller survives long enough to refresh state

