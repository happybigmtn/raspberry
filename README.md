<div align="left" id="top">
<a href="https://docs.fabro.sh"><img alt="Fabro" src="docs/logo/dark.svg" height="75"></a>
</div>

## Fork of Fabro for repo-level autonomy

This repository is a working fork of [Fabro](https://github.com/fabro-sh/fabro).
We forked it because Fabro is already a strong workflow engine for individual
agent runs, but we wanted to push it further into repo-level supervision:
multi-lane frontier management, autonomous replay and recovery, and a live
human dashboard for ongoing runs.

[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE.md)
[![docs](https://img.shields.io/badge/docs-fabro.sh-357F9E)](https://docs.fabro.sh)

## What Fabro does

Upstream Fabro is the execution substrate:

- workflow graphs written as code
- staged agent runs with deterministic gates
- checkpointed run state and artifact output
- synthesis tools for generating checked-in `fabro/` packages from blueprints

If you want the original project and its broader documentation, start here:

- upstream repo: [fabro-sh/fabro](https://github.com/fabro-sh/fabro)
- upstream docs: [docs.fabro.sh](https://docs.fabro.sh)

## Why this fork exists

This fork is focused on a different question:

> What does it take to supervise a whole repository frontier, not just launch
> isolated workflow runs?

That means adding a control plane above Fabro that can:

- evaluate many lanes across one repo
- decide what is ready, blocked, running, failed, or complete
- replay or regenerate failed work automatically
- keep durable repo truth in sync with human-facing coordination surfaces

## What this fork adds

### Raspberry

Raspberry is the repo-level supervisory plane layered on top of Fabro.

It adds:

- `fabro/programs/*.yaml` program manifests
- lane readiness and dependency evaluation
- bounded `raspberry execute` and `raspberry autodev` loops
- durable runtime truth in `.raspberry/*`
- blueprint-first repo synthesis through `fabro synth import/create/evolve`
- stronger implementation-family review, quality, and promotion contracts

The core Raspberry commands are:

```bash
raspberry plan --manifest fabro/programs/<program>.yaml
raspberry status --manifest fabro/programs/<program>.yaml
raspberry watch --manifest fabro/programs/<program>.yaml
raspberry execute --manifest fabro/programs/<program>.yaml
raspberry autodev --manifest fabro/programs/<program>.yaml
```

### Paperclip

Paperclip is the coordination and dashboard layer on top of Raspberry.

It adds:

- a browser-based dashboard for live frontiers
- synchronized issue/work-product views of repo progress
- native wake/refresh loops for the Raspberry orchestrator
- a human-friendly surface for inspection, coordination, and handoff

The main Paperclip commands are:

```bash
fabro paperclip bootstrap --target-repo /path/to/repo --program <program>
fabro paperclip status --target-repo /path/to/repo --program <program>
fabro paperclip wake --target-repo /path/to/repo --program <program> --agent raspberry-orchestrator
fabro paperclip refresh --target-repo /path/to/repo --program <program>
```

After bootstrap or start, the preferred human dashboard is the Paperclip web
UI served by the local instance, typically at `http://127.0.0.1:3100/`.

## Current status

This fork is still a work in progress.

The core direction is stable:

- Fabro remains the workflow engine
- Raspberry remains the execution supervisor
- Paperclip remains the human dashboard and coordination surface

But the implementation is still evolving, especially around:

- run recovery and failure classification
- review and promotion workflow contracts
- Codex/Claude local auth handling
- Paperclip sync and live dashboard behavior

Expect active iteration rather than a frozen platform contract.

## Mental model

```text
specs / plans / doctrine
          |
          v
 blueprint + checked-in fabro/ package
          |
          v
      Raspberry
          |
          +--> outputs/**/*
          +--> .raspberry/*-state.json
          +--> ~/.fabro/runs/<run-id>
          |
          v
      Paperclip web
```

## Getting started

Install the Fabro toolchain:

```bash
# With Claude Code
curl -fsSL https://fabro.sh/install.md | claude

# With Codex
codex "$(curl -fsSL https://fabro.sh/install.md)"

# With Bash
curl -fsSL https://fabro.sh/install.sh | bash
```

Then initialize or evolve a repo package:

```bash
fabro install
fabro synth create --target-repo /path/to/repo --program <program>
raspberry plan --manifest /path/to/repo/fabro/programs/<program>.yaml
```

For the fork-specific surfaces, start with:

- [From Specs to Blueprints](https://docs.fabro.sh/guides/from-specs-to-blueprint)
- [Raspberry Supervisory Plane](https://docs.fabro.sh/reference/raspberry)
- [Paperclip Coordination](https://docs.fabro.sh/reference/paperclip)
- [Raspberry Operator Runbook](https://docs.fabro.sh/guides/raspberry-operator-runbook)

---

## Help or Feedback

- [Bug reports](https://github.com/fabro-sh/fabro/issues) via GitHub Issues
- [Feature requests](https://github.com/fabro-sh/fabro/discussions) via GitHub Discussions
- Email [bryan@qlty.sh](mailto:bryan@qlty.sh) for questions
- See [CONTRIBUTING.md](CONTRIBUTING.md) for build instructions and development workflow

---

## License

Fabro is licensed under the [MIT License](LICENSE.md).
