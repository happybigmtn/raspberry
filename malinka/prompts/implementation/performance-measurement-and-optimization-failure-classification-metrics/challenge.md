# Failure Classification Metrics Lane — Challenge

Perform a cheap adversarial review of the current slice for `performance-measurement-and-optimization-failure-classification-metrics` before the expensive final review runs.

Your job is to challenge assumptions, find obvious scope drift, identify weak proof, and catch mismatches between code and artifacts. Do not bless the slice as merge-ready; that belongs to the final review gate.


Verification artifact must cover
- summarize the automated proof commands that ran and their outcomes

Layout/domain invariant checklist (flag every missing item in `.fabro-work/verification.md`):
- layout invariant test proving the rendered board/grid contains no duplicate domain values

Structural discipline
- if a new source file would exceed roughly 400 lines, split it before landing
- do not mix state transitions, input handling, rendering, and animation in one new file unless the prompt explicitly justifies that coupling
- if the slice cannot stay small, stop and update the artifacts to explain the next decomposition boundary instead of silently landing a monolith

Challenge checklist:
- Is the slice smaller than the plan says, or larger?
- Did the implementation actually satisfy the first proof gate?
- Are any touched surfaces outside the named slice?
- Are the artifacts overstating completion?
- Are the tests actually verifying behavioral outcomes, or are they trivial stubs that pass without real logic?
- Is there an obvious bug, trust-boundary issue, or missing test the final reviewer should not have to rediscover?

Write a short challenge note in `.fabro-work/verification.md` or amend it if needed, focusing on concrete gaps and the next fixup target. Do not write `promotion.md` here.
