The artifacts are polished and ready. Here's a summary of what was improved:

## Changes Made

### `spec.md`
- **Tightened architecture section**: Each of the 4 components (scaffold ordering, bootstrap gate, asset resolution, quality checks) is now clearly marked with its exact file/line reference and implementation status
- **Fixed status accuracy**: Runtime-stable asset refs changed from "✅ Implemented" to "⚠️ Convention only" to accurately reflect that only the `@malinka/prompts/` naming convention exists — no explicit copy/validation
- **Test coverage accuracy**: Distinguishes clearly between existing tests (4 named, confirmed present) and required tests (4 named, confirmed absent per grep)
- **Cleaner acceptance table**: Precise line references for all implemented features; gaps clearly labeled as "NOT Implemented" with explicit "Blocked" notes
- **Implementation phases**: Checklist format showing Phase 0 in progress vs Phase 1 hardening items

### `review.md`
- **Verdict up front**: "Phase 0 gate: NOT READY" with a one-line summary — no hunting for the conclusion
- **"What Can Go Wrong" section**: Four concrete failure scenarios replacing the more abstract danger assessment tables
- **Pass 1 / Pass 2 structure**: Cleaner and more direct — trust boundaries, dangerous actions, then coupled-state analysis
- **Privilege escalation table**: Separated from dangerous actions; path traversal gap clearly flagged
- **Remaining blockers**: Single prioritized table with severity, status, and explicit "Required" for all Phase 0 gate items
- **Recommendations**: Actionable items with specific commands (not just vague suggestions)