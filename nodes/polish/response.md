Both artifacts are polished and written. Here's a summary of what changed and why:

---

## Changes Made

### `spec.md` — Structural Rewrite

| Area | Before | After |
|------|--------|-------|
| **Current State tables** | Render.rs table listed line numbers that were slightly off and didn't distinguish severity | Rewrote with confirmed line numbers (e.g., 2015–2020, 2487–2491) and explicit severity ratings; added `gpt-5.4` vs `gpt-5.3-codex` discrepancy as a named open question |
| **`cli_failure_is_retryable_for_fallback()`** | Described only in prose | Quoted the actual function body verbatim so implementation matches exactly; added the auth/transient conflation bug with line-level citation |
| **`DispatchOutcome` / `AutodevCycleReport`** | Described generically | Quoted actual struct field lists from `dispatch.rs:17-24` and `autodev.rs:96-109`, showing exactly what's missing |
| **Architecture contracts** | Stated as abstract rules | Added concrete Rust type definitions for `DispatchOutcome` extension (`ProviderHealth` enum, `fallback_reason`) and `AutodevCycleReport` extension (`ProviderCycleUsage` struct) |
| **Open Questions** | None listed | Three named open questions: `gpt-5.4` vs `gpt-5.3-codex`, MiniMax/Kimi quota strings, chain exhaustion behavior — all blocking Milestone 1 or 4 |
| **Non-Goals** | Generic | Explicitly excludes plan 003 exec-path work |

### `review.md` — Evidence-Driven Security Narrative

| Area | Before | After |
|------|--------|-------|
| **Source citations** | Generic references | Line-level citations for every claim (`cli.rs:800-810`, `dispatch.rs:17-24`, `autodev.rs:96-109`, `error.rs:212`, `render.rs:2015-2019`) |
| **Auth/transient conflation** | Not explicitly called out as a bug | Added a dedicated **Dangerous Actions** row and a quoted code block showing `"401 unauthorized"` and `"timed out"` in the same OR chain as quota signals |
| **Test assertions** | Not mentioned | Called out explicitly: tests in `render.rs` validate the **wrong behavior** and must be updated alongside the fix |
| **Chain exhaustion** | Not mentioned | Added as a High-severity risk: `stage_dir` is ephemeral, `provider_used.json` is lost across cycles, no aggregate path exists |
| **Final verdict conditions** | Three generic conditions | Three specific conditions tied to the three open questions, each with a concrete artifact or action required |
| **Evidence log** | Basic table | Expanded to include all verification methods and confirmed line numbers |