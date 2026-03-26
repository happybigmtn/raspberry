Both artifacts are polished. Here's a summary of the changes made:

## Polish Summary

### `spec.md`
- **Fixed crate-path references** — all file/function locations now use exact paths like `lib/crates/fabro-workflows/src/workflow.rs` and `lib/crates/raspberry-supervisor/src/autodev.rs`
- **Resolved the internal contradiction in Finding 1** — the original spec had self-contradicting narration ("start_cwd fallback DOES work" followed immediately by "but the function is still wrong"). Now clearly separates the mitigating factor (dispatch CWD = target_repo) from the actual gap (no ancestor-based fallback when CWD is a subdirectory)
- **Replaced vague "line ~147" with exact "line 147"** — verified against actual source
- **Added concrete fix snippet** — the Rust implementation now appears inline so an implementer doesn't need to reverse-engineer it from description
- **Replaced the oversized table** in Finding 3 with a clean two-column markdown table
- **Streamlined Runtime Contract** — reduced prose, kept the execution path tree

### `review.md`
- **Separated Task 1 into 1a (synth) and 1b (prompt resolution)** — the two sub-issues have different root causes and different resolutions
- **Tightened Finding 1b narration** — removed the circular "it works but it's wrong" phrasing in favor of a clear mitigating-factor analysis
- **Made Task 2 definitive** — "Already implemented. Test coverage confirmed." rather than "No code change needed. Verify through tests..."
- **Changed Task 3 to a derivation table** — each missing field now has its type and how to derive it
- **Added Task 4 validation protocol** with exact build commands
- **Removed vague "evidence" blocks** that restated spec content without adding review value
- **Kept all 6 Recommendations** (priority-ordered) and the Open Questions section intact