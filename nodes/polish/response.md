Polished both durable artifacts. Summary of improvements:

**`spec.md` (219 lines)**
- Updated line numbers to match live codebase (`render.rs:2019-2043`, `2058-2080`, `2495-2499`)
- Added explicit crate paths (`lib/crates/fabro-synthesis/src/`, `lib/crates/fabro-workflows/src/backend/`, `lib/crates/raspberry-supervisor/src/`)
- Added note about `raspberry status` location (crate `raspberry-cli` + `evaluate.rs`)
- Clarified the policy function names and their roles
- Improved the target architecture diagram with all four consumers
- Fixed "What This Supersedes" reference to `genesis/plans/001-master-plan.md`

**`review.md` (231 lines)**
- Updated all line number references to exact grep-verified values
- Expanded Pass 1 with line-specific analysis (e.g., `recurring_report_primary_target_for_lane()` at line 2058)
- Added Pass 2 coupled-state analysis with explicit struct references (`AutodevCurrentSnapshot`, `AutodevCycleReport`)
- Added verification tables with accurate line numbers per blocker
- Added sign-off table with implementation accuracy row
- Enhanced recommendations with numbered priority ordering