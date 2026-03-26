# Artifact Validation Lane Codex Unblock — Challenge

Perform a cheap adversarial review of the current slice for `parent-review-gauntlet-rollout-artifact-validation-codex-unblock` before the expensive final review runs.

Your job is to challenge assumptions, find obvious scope drift, identify weak proof, and catch mismatches between code and artifacts. Do not bless the slice as merge-ready; that belongs to the final review gate.


Verification artifact must cover
- summarize the automated proof commands that ran and their outcomes

Security edge checklist (flag every missing item in `.fabro-work/verification.md`):
- wrong-seed verification test proving verify_house rejects mismatched randomness
- missing-seed rejection test proving the round cannot advance without house-authorized randomness
- player-bypass test proving apply_player cannot synthesize or substitute house randomness

Challenge checklist:
- Is the slice smaller than the plan says, or larger?
- Did the implementation actually satisfy the first proof gate?
- Are any touched surfaces outside the named slice?
- Are the artifacts overstating completion?
- Are the tests actually verifying behavioral outcomes, or are they trivial stubs that pass without real logic?
- Is there an obvious bug, trust-boundary issue, or missing test the final reviewer should not have to rediscover?

Write a short challenge note in `.fabro-work/verification.md` or amend it if needed, focusing on concrete gaps and the next fixup target. Do not write `promotion.md` here.
