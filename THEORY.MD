# Theory: Keep Fabro and Raspberry Continuously Generating Work

## Problem

Fabro and Raspberry were still capable of looking "done" or "stuck" for the
wrong reasons. The first bug was frontier exhaustion: once the currently
rendered manifest ran out of runnable lanes, the system could go idle even when
repo doctrine still described obvious unfinished work. The second bug was truth
loss: the operator-facing state could collapse real failure causes into generic
messages like "script failed with exit code: 1" or accidentally trigger deeper
work just to render a status table.

Zend and Myosu make the distinction concrete. Zend needed to keep moving past
bootstrap and to classify implementation-child failures into meaningful next
actions. Myosu needed read-only top-level status to stay fast and truthful
without silently evaluating child programs deeply enough to look like a hidden
scheduler.

## Operating Theory

The no-idle loop only works if authorship, orchestration, and truth all reinforce
each other.

Authorship lives in `fabro-synthesis`. `author_blueprint_for_evolve(...)` must
be able to materialize missing parent-program work from doctrine instead of only
re-importing the current package. The parent program remains the roadmap, while
implementation child programs remain execution fronts inside that roadmap.

Orchestration lives in `raspberry-supervisor::autodev`. Spare capacity and
doctrine deltas should be reasons to search for more work, but only inside an
explicit frontier budget. Otherwise a single empty slot turns into unbounded
frontier spam.

Truth lives in `program_state.rs` and `evaluate.rs`. Those surfaces must be
child-state-authoritative when child state already exists, because read-only
commands like `raspberry status` should not become implicit work runners. The
same surfaces also need richer failure extraction than the final top-level
workflow error alone, because the actionable cause often appears earlier in the
run history than the final collapse message.

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

The newest execution step extends that into actual remediation for one concrete
resource family. Zend-style service/interface runs already know how to honor
`ZEND_BIND_PORT` and `ZEND_DAEMON_URL`, so Raspberry can lease a daemon port,
pass those env vars through `fabro run`, and restart the active project loops
on the updated binaries instead of leaving resource-collision handling as a
future note.

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

## Open Questions

The main remaining open question is the live proving strategy. The watchdog
loops on Zend and Myosu are active, so there is still a coordination question
around how to run explicit bounded proving passes without colliding with the
existing overnight controllers.

The other open question is how far to push failure extraction beyond the current
run-history heuristics. The present approach is already much better than the
generic wrapper messages, but some future cases may need even more structured
engine output if we want classification to stay honest without pattern-matching
ever-larger error strings.
