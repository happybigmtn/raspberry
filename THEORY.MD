# Theory: Keep Fabro and Raspberry Continuously Generating Work

## Problem

Fabro and Raspberry were still capable of looking "done" or "stuck" for the
wrong reasons. The first bug was frontier exhaustion: once the currently
rendered manifest ran out of runnable lanes, the system could go idle even when
repo doctrine still described obvious unfinished work. The second bug was truth
loss: the operator-facing state could collapse real failure causes into generic
messages like "script failed with exit code: 1" or accidentally trigger deeper
work just to render a status table. The newest proving ground, `rXMRbro`, adds
two more classes of false failure: child programs can die before domain work
even starts because the runtime launch path itself assumes an interactive shell
and a writable home-directory log location, and otherwise healthy lanes can
pollute themselves during the contract stage by creating durable artifacts too
early.

Zend and Myosu made the orchestration/truth distinction concrete. `rXMRbro`
adds the next requirement: a zero-human system cannot depend on incidental host
shell behavior or on `~/.fabro/logs` being writable inside nested worker
contexts.

## Operating Theory

The no-idle loop only works if authorship, orchestration, truth, and launch
hygiene all reinforce each other.

Authorship lives in `fabro-synthesis`. `author_blueprint_for_evolve(...)` must
be able to materialize missing parent-program work from doctrine instead of only
re-importing the current package. The parent program remains the roadmap, while
implementation child programs remain execution fronts inside that roadmap.

Orchestration lives in `raspberry-supervisor::autodev` and `dispatch.rs`.
Spare capacity and doctrine deltas should be reasons to search for more work,
but only inside an explicit frontier budget. Otherwise a single empty slot
turns into unbounded frontier spam. The same layer must also launch work in a
way that is invariant to dotfiles and shell interactivity. If dispatch uses
`bash -ic`, then the scheduler is no longer scheduling work; it is also
implicitly depending on starship prompts, tty behavior, and whatever the host
shell startup scripts happen to do.

Truth lives in `program_state.rs` and `evaluate.rs`. Those surfaces must be
child-state-authoritative when child state already exists, because read-only
commands like `raspberry status` should not become implicit work runners. The
same surfaces also need richer failure extraction than the final top-level
workflow error alone, because the actionable cause often appears earlier in the
run history than the final collapse message. In `rXMRbro`, the actionable cause
was not "provably fair failed" or "house agent failed"; it was that the
framework launched nested work through an interactive shell and a logging path
that panicked on a read-only filesystem. The newer truth discovery is that
`conclusion.json` and `status.json` must stay authoritative even if the
progress log ends with `WorkflowRunCompleted`; otherwise a genuinely failed run
can be re-labeled as successful and lose the recovery metadata needed for
self-healing. The related synthesis truth is that stage-local prompts cannot
blindly inherit every lane-level artifact instruction. If the contract stage
sees `Required durable artifacts: spec.md, review.md` in the same context that
tells it to write only `.fabro-work/contract.md`, the framework has created an
ambiguous contract and the model will sometimes obey the wrong part.

## Strategy

The strategy is to deepen the existing seams rather than create a new control
plane.

In `planning.rs`, evolve now reuses doctrine-derived create-time authorship to
merge missing parent units while leaving implementation child programs scoped to
their own reviewed contract.

In `autodev.rs`, the loop now uses per-program doctrine fingerprints and a
bounded frontier budget. Doctrine-triggered evolve only happens when the
frontier is settled or genuinely starved, not when ready work is already queued.

In `program_state.rs`, child-program refresh now prefers existing child runtime
state over recursive child evaluation. That keeps parent truth fast enough for
read-only commands. The same file also now merges failure evidence more
carefully: generic wrapper failures yield to richer stage failures, and cycle
wrapper messages yield to concrete underlying stage errors when those exist.
It also normalizes persisted paths during refresh so the state files themselves
do not accumulate pathological relative-path chains over repeated evaluations.

In `failure.rs` and `evaluate.rs`, the shared failure kind plus recovery action
now flow all the way to operator output. The point is not taxonomy for its own
sake; it is to let the status surface answer "what happened?" and "what should
happen next?" from the same source of truth.

The newest runtime hardening step is to remove unnecessary shell mediation and
make logging opportunistic rather than fatal. `dispatch.rs` and the
`synth import/evolve` path in `autodev.rs` should launch `fabro` directly with
explicit env vars instead of through `bash -ic`. `fabro-cli::logging` should
prefer file logs when it can create them, but it must fall back to run-local
logging when the home directory is unavailable or read-only. That keeps nested
autonomous runs from failing before their first stage executes.

The newest execution step extends that into actual remediation for one concrete
resource family. Zend-style service/interface runs already know how to honor
`ZEND_BIND_PORT` and `ZEND_DAEMON_URL`, so Raspberry can lease a daemon port,
pass those env vars through `fabro run`, and restart the active project loops
on the updated binaries instead of leaving resource-collision handling as a
future note.

The newest workflow hardening step follows the same pattern: make the contract
stage explicit and self-policing. The prompt should state that durable lane
artifacts belong to later stages, and the deterministic `contract_check` gate
should fail if the contract-stage commit touched anything except
`.fabro-work/contract.md`. That turns ambiguous stage behavior into a tight
retry loop instead of letting stray `spec.md` or `review.md` files poison the
rest of the lane.

## Key Discoveries

The first spare-capacity evolve pass was too eager. Treating any doctrine delta
as permission to evolve even when ready work already existed created churn
instead of forward motion. The trigger had to be tightened to starvation or
settlement.

The live Myosu repo exposed that parent status was still too expensive even
after the main no-idle changes. The culprit was not the status table itself; it
was parent runtime refresh falling through to deep child evaluation. Splitting a
local evaluation path for read-only CLI surfaces and preferring stored child
state fixed that.

The live Zend child runs showed that the final workflow failure is often not the
best explanation. Hermes was really an integration conflict, private control
plane was really a stall watchdog, and command-center-client was really an
environment collision repeatedly wrapped in later cycle-collapse messages. The
right fix was better failure extraction plus better classification ordering, not
more generic labels.

Another useful discovery is that user-visible implementation lanes need more
than proof commands. Reviewed slice docs often already contain explicit smoke
gates, and carrying those into generated verify commands makes settlement more
honest for interface and service work.

One more operational discovery is that structural truth matters alongside
semantic truth. The live Zend child state eventually exposed absurd repeated
`../../` chains in persisted `run_config` paths. That was not just ugly output;
it was a sign that state refresh was not rewriting normalized path truth back to
disk. Fixing that keeps the durable state readable and prevents future tooling
from inheriting junk paths.

Another discovery is that the current active Zend failures did not actually need
a full general-purpose resource scheduler to improve. The repo-level scripts had
already been written to accept leased daemon env, which meant the framework only
needed one small bridge: resource lease allocation in Raspberry plus host-env
passthrough in `fabro run`.

The newest and most important discovery from `rXMRbro` is that some "failed
implementation" lanes were not implementation failures at all. The child
autodev evidence showed `bash: cannot set terminal process group`, starship
startup noise, and then a panic from `tracing-appender` trying to create
`~/.fabro/logs` on a read-only filesystem. That changed the theory
substantially. The right lever is not better prompts or different repo plans;
it is launch-path determinism in the framework itself.

The newest contract-stage discovery is that fresh clean runs still wrote stray
artifact files during `contract`, including one case that wrote `spec.md` and
`review.md` into the wrong lane entirely. The prompt text was not actually
contradiction-free: the shared Goal block still advertised required durable
artifacts before the contract instructions said "write the contract only." The
right fix is not to hope the model prioritizes correctly; it is to remove the
ambiguity in the contract prompt and add a gate that rejects any non-contract
write from the contract-stage commit.

Another important discovery is that local-only direct integration has two
different failure modes. The first was missing `origin`, which made trunk
detection and push-based settlement impossible. The second was subtler: once
the system fell back to a local branch like `main`, linked-worktree repos could
still fail because `git branch -f main HEAD` refuses to move a branch that is
checked out in the live repo worktree. For unattended local proving grounds,
that branch-ref update cannot stay a hard stop. The framework now needs a
best-effort local settlement mode that leaves the failure actionable instead of
terminal.

The latest Myosu-specific discovery is that not every remaining proving-ground
failure needs a brand-new recovery family. Two live failures that were still
blocking bounded runs — `goal gate unsatisfied for node verify and no retry
target` and a 600000ms scripted test timeout — were really proof/verify
failures that had fallen through to `unknown`. Expanding the proof-script
classifier to cover those strings was enough to turn them into
`regenerate_lane` work instead of `surface_blocked`.

The newest workflow-design discovery is that implementation review had become
structurally muddled. `Settle` was acting as a hidden subjective signoff stage,
while `Review` was either missing or underused for implementation-family work.
The cleaner model is: deterministic checks (`Verify`, `Quality`) feed `Fixup`,
then one explicit stronger-model `Review` stage owns merge-readiness and folds
security scrutiny into that judgment when the slice touches trust boundaries.
That keeps the workflow legible: deterministic loop first, subjective gate
second, contract enforcement last.

## Open Questions

The main remaining open question is the live proving strategy. The watchdog
loops on Zend, Myosu, and now `rXMRbro` are active, so there is still a
coordination question around how to run explicit bounded proving passes without
colliding with the standing controllers.

The other open question is how far to push runtime isolation. Direct command
launch plus non-fatal logging should remove the current shell/home-directory
footguns, but nested autonomous execution may still surface other host-coupled
assumptions over time. The right long-term answer may be a stricter framework
contract for child-program execution environments, not just a growing list of
patched symptoms.

The remaining workflow question is whether stage-specific goal context should
eventually become first-class instead of being simulated with stronger prompt
text and tighter gates. The current fix should stop contract-stage drift, but
`rXMRbro` is also a reminder that one lane-level goal string can mean different
things at different stages, and the framework may eventually need to model that
directly.
