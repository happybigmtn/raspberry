## Raspberry: Autodev Orchestrator

This repository is a working fork of [Fabro](https://github.com/fabro-sh/fabro) and [Paperclip](https://github.com/paperclipai/paperclip).
We forked it because Fabro is already a strong workflow engine for individual
agent runs, but we wanted to push it further into repo-level supervision:
multi-lane frontier management, autonomous replay and recovery, and a live
human dashboard for ongoing runs. Paperclip is a great agent orchestrator and command center. If you combined both with a supervisory brain, great things can result. 

[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE.md)

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

Upstream project:

- [paperclipai/paperclip](https://github.com/paperclipai/paperclip)

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

## How the layers connect

The key integration in this fork is:

- Fabro defines and executes workflow graphs
- Raspberry evaluates repo-level frontier state and decides what should run
- Paperclip mirrors that live frontier into a browser-based coordination surface

In practice, that means we are not using Paperclip as a second scheduler.
Paperclip is the dashboard and wake surface, while Raspberry stays the source
of execution truth and Fabro stays the workflow engine underneath it.

### Architecture

```mermaid
flowchart TD
    A[Specs / Plans / Doctrine] --> B[Fabro Blueprints]
    B --> C[Checked-in fabro/ Package]
    C --> D[Fabro Runs]
    C --> E[Raspberry Supervisor]
    D --> F[outputs/**/*]
    D --> G[~/.fabro/runs/<run-id>]
    E --> H[.raspberry/*-state.json]
    E --> I[Lane readiness / retries / autodev]
    E --> J[Paperclip refresh + wake]
    J --> K[Paperclip bundle]
    K --> L[Paperclip web dashboard]
    E --> L
```

### Control Loop

```mermaid
sequenceDiagram
    participant Human
    participant Paperclip
    participant Raspberry
    participant Fabro
    participant Repo

    Human->>Paperclip: inspect dashboard / wake orchestrator
    Paperclip->>Raspberry: heartbeat / route trigger
    Raspberry->>Repo: read manifest, artifacts, runtime state
    Raspberry->>Fabro: dispatch ready lane
    Fabro->>Repo: run workflow, write outputs
    Fabro->>Repo: persist run truth in ~/.fabro/runs
    Raspberry->>Repo: persist frontier truth in .raspberry/*
    Raspberry->>Paperclip: refresh synced frontier/issues/work products
    Paperclip->>Human: updated web dashboard
```

### What is actually linked

- `fabro/programs/*.yaml` gives Raspberry a repo-level manifest to supervise.
- `fabro synth import/create/evolve` keeps the checked-in control plane in sync
  with doctrine and run evidence.
- Raspberry writes durable frontier truth to `.raspberry/*` and drives bounded
  `execute` / `autodev` loops.
- `fabro paperclip bootstrap/refresh` exports that frontier into a Paperclip
  company bundle, synced issues, documents, work products, and attachments.
- `fabro paperclip wake --agent raspberry-orchestrator` lets Paperclip trigger
  the Raspberry control loop without becoming the scheduler itself.

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

- [From Specs to Blueprints](./docs/guides/from-specs-to-blueprint.mdx)
- [Raspberry Supervisory Plane](./docs/reference/raspberry.mdx)
- [Paperclip Coordination](./docs/reference/paperclip.mdx)
- [Raspberry Operator Runbook](./docs/guides/raspberry-operator-runbook.mdx)

---

## Help or Feedback

- Email [happygrandmtn@gmail.com](mailto:happygrandmtn@gmail.com) for questions
---

## License

Fabro is licensed under the [MIT License](LICENSE.md).
