# Autodev Runbook

This is the single file to point coding agents at when they need to launch or
repair a Fabro/Raspberry autodev controller in another repo.

It reflects the current operator workflow that actually works in practice:

- build local `fabro`/`raspberry` binaries from this repo
- run autodev from a disposable controller checkout, not from a human checkout
- use an allowed controller branch name
- reset the controller to `origin/main`, regenerate, and commit the generated
  package refresh before launch
- prefer an attached/PTy launch when you need the process to stay alive under an
  agent shell

## 1. Build Fabro First

From `/home/r/coding/fabro`:

```bash
cargo build -p fabro-cli -p raspberry-cli --target-dir target-local
```

Use these binaries:

- `/home/r/coding/fabro/target-local/debug/fabro`
- `/home/r/coding/fabro/target-local/debug/raspberry`

## 2. Never Launch From A Dirty Human Checkout

Autodev should run from a disposable controller checkout, not from the repo a
human is editing.

Use a sibling worktree on an allowed controller branch:

- `autodev/main`
- `autodev-main`

Do not use names like `autodev/main-validate`; current freshness checks reject
them.

Example controller creation from a repo with default branch `main`:

```bash
git -C /path/to/repo worktree add -b autodev/main /path/to/repo-autodev origin/main
```

If `autodev/main` is already taken by another worktree, use:

```bash
git -C /path/to/repo worktree add -b autodev-main /path/to/repo-autodev origin/main
```

## 3. Refresh The Controller Before Every Launch

From the controller checkout:

```bash
git fetch origin
git reset --hard origin/main
git clean -fd
```

Then regenerate from the checked-in blueprint:

```bash
/home/r/coding/fabro/target-local/debug/fabro --no-upgrade-check synth create \
  --target-repo /path/to/repo-autodev \
  --program <program> \
  --blueprint /path/to/repo-autodev/malinka/blueprints/<program>.yaml \
  --no-decompose \
  --no-review
```

Then commit the generated package refresh on the disposable controller branch:

```bash
git add -A
git -c user.name=Fabro -c user.email=noreply@fabro.sh \
  commit -m "fabro: refresh generated package"
```

If there is nothing to commit, that is fine.

## 4. Launch Autodev

Canonical launch:

```bash
/home/r/coding/fabro/target-local/debug/raspberry autodev \
  --manifest /path/to/repo-autodev/malinka/programs/<program>.yaml \
  --fabro-bin /home/r/coding/fabro/target-local/debug/fabro \
  --max-parallel 10 \
  --max-cycles 0 \
  --poll-interval-ms 1000 \
  --evolve-every-seconds 0
```

### Important launch note

If an agent shell cannot keep background children alive reliably, do not use
`nohup ... &` and assume the controller will survive. Launch it in an attached
PTY/session and monitor it there.

## 5. Monitor The Live Controller

Primary report:

```bash
sed -n '1,120p' /path/to/repo-autodev/.raspberry/<program>-autodev.json
```

Primary state file:

```bash
sed -n '1,120p' /path/to/repo-autodev/.raspberry/<program>-state.json
```

The fields that matter first are:

- `ready`
- `running`
- `failed`
- `complete`
- `stop_reason`

For current Fabro builds, also inspect cycle telemetry:

- `oldest_running_seconds`
- `running_without_completion_cycles`
- `ready_while_saturated`

Interpretation:

- `running=10` and `ready>0` means the controller is saturated
- `running_without_completion_cycles` climbing means the wave is not landing
  completions
- `ready_while_saturated=true` means there is backlog waiting behind pinned slots

## 6. Common Failure Modes

### Background controller dies after a few cycles

Symptom:

- lock file exists
- PID in the lock is dead
- log only shows a few cycles

Meaning:

- the controller was launched in a way that did not survive the agent shell

Fix:

- relaunch in an attached PTY/session

### Controller branch diverged from origin

Symptom:

- autodev logs: `local default branch diverged from origin (ahead X, behind Y)`

Meaning:

- you committed generated controller state locally, then `origin/main` moved

Fix:

```bash
git fetch origin
git reset --hard origin/main
git clean -fd
/home/r/coding/fabro/target-local/debug/fabro --no-upgrade-check synth create \
  --target-repo /path/to/repo-autodev \
  --program <program> \
  --blueprint /path/to/repo-autodev/malinka/blueprints/<program>.yaml \
  --no-decompose \
  --no-review
git add -A
git -c user.name=Fabro -c user.email=noreply@fabro.sh \
  commit -m "fabro: refresh generated package"
```

Then relaunch autodev.

### Run-branch push conflict

Symptom:

- lane fails on push to `fabro/run/<run-id>`
- error is non-fast-forward

Meaning:

- a stale remote checkpoint branch exists for that run ID

Fix:

1. Delete the stale remote run branch:

```bash
git -C /path/to/repo-autodev push origin :refs/heads/fabro/run/<run-id>
```

2. Clear the stale local failed lane record if it remains pinned in
   `surface_blocked`
3. Restart the controller so it re-evaluates from repaired remote state

### Controller alive but not landing completions

Symptom:

- `running=10`
- `complete` flat
- `running_without_completion_cycles` increasing

Meaning:

- launch path is healthy, but long-lived runs are pinning all capacity

What to inspect:

- running lane list in `.raspberry/<program>-state.json`
- each run directory in `/home/r/.fabro/runs/<date>-<run-id>/`
- stage labels like `Fixup`, `Review`, `Challenge`, `Deep Review`

## 7. One-Line Rule

If autodev looks strange:

1. stop assuming the controller checkout is healthy
2. rebuild `fabro`
3. reset controller to `origin/main`
4. regenerate from blueprint
5. recommit generated refresh
6. relaunch from an allowed controller branch
