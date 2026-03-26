# Fabro / Raspberry Specification

Status: Active
Date: 2026-03-26
Type: Capability Spec

## What This Project Is

Fabro is an open-source AI workflow orchestration engine written in Rust. It compiles Graphviz-format workflow graphs into staged, checkpointed agent runs with tool use, retry, and artifact production. Raspberry is a supervisory plane layered on top of Fabro that turns a corpus of planning documents into autonomously-executing, multi-lane programs with adversarial review, direct trunk integration, and continuous frontier management.

The combined system turns `plans/*.md` into landed code on `main` without human intervention per-lane.

## Who It's For

A technical operator managing multiple AI-supervised codebases. They write planning documents (specs, plans, numbered milestones) and expect the system to decompose those plans into executable work, dispatch AI agents, review results adversarially, and integrate passing work to trunk. The operator monitors progress via TUI or Paperclip dashboard and intervenes only for blocked work or policy decisions.

## Architecture

```
                    OPERATOR
                       |
          specs/ plans/ SPEC.md
                       |
                       v
            +---------------------+
            |  fabro synth create |  (Opus-decomposed plan → blueprint → package)
            +----------+----------+
                       |
                       v
            +---------------------+
            |  malinka/ package   |  (programs, workflows, run-configs, prompts)
            +----------+----------+
                       |
                       v
            +---------------------+
            |  raspberry autodev  |  (evaluate → dispatch → watch → settle cycle)
            +----------+----------+
                       |
          +------------+------------+
          |            |            |
          v            v            v
    +-----------+ +-----------+ +-----------+
    | Lane: impl| | Lane: impl| | Lane: rev |
    | (MiniMax) | | (MiniMax) | | (Kimi)    |
    +-----------+ +-----------+ +-----------+
          |            |            |
          v            v            v
    +-----------+ +-----------+ +-----------+
    | verify +  | | verify +  | | adjudicate|
    | quality   | | quality   | | + score   |
    +-----------+ +-----------+ +-----------+
          |            |            |
          v            v            v
    +--------------------------------------+
    |  direct trunk integration            |
    |  (squash-merge to main, artifact)    |
    +--------------------------------------+
          |
          v
    +---------------------+
    |  Paperclip sync     |  (dashboard, issues, work products)
    +---------------------+
```

## Tech Stack

### Rust (core engine)
- **28 crates** in `lib/crates/`, ~206K LOC
- Async runtime: `tokio`
- HTTP server: `axum` (fabro-api)
- CLI: `clap`
- Serialization: `serde`, `serde_json`, `serde_yaml`
- Graph parsing: custom Graphviz parser (`fabro-graphviz`)
- LLM clients: `reqwest` + streaming for Anthropic, OpenAI, Gemini, MiniMax
- Database: `rusqlite` with WAL mode (`fabro-db`)
- TUI: `ratatui` + `crossterm` (`raspberry-tui`)
- OpenAPI types: `typify` code generation (`fabro-types`)
- Telemetry: Segment analytics + Sentry crash reporting (`fabro-telemetry`)

### TypeScript (web frontend)
- `apps/fabro-web`: React 19 + React Router + Vite + Tailwind CSS
- `lib/packages/fabro-api-client`: Auto-generated Axios client from OpenAPI spec
- `apps/marketing`: Astro static site (fabro.sh)

### Key External Services
- LLM providers: Anthropic (Claude), OpenAI (GPT/Codex), MiniMax, Kimi, Gemini
- Paperclip: Local coordination dashboard server
- GitHub: App-based auth for PR creation (`fabro-github`)
- SSH: Remote sandbox execution (`fabro-exe`)

## Key Design Decisions Already Made

1. **Plan-first execution** — plans/*.md are the primary work objects, not ad-hoc agent prompts
2. **Graphviz workflow graphs** — stages and transitions as DOT attributes, parsed by `fabro-graphviz`
3. **Sandbox trait abstraction** — uniform interface for local, Docker, SSH, Sprites, Daytona execution
4. **OpenAPI-first API** — `fabro-api.yaml` drives both Rust and TypeScript type generation
5. **Checkpoint/resume** — workflows can pause, checkpoint state, and resume from any stage
6. **Blueprint → renderer pipeline** — `fabro-synthesis` compiles blueprints deterministically into package artifacts
7. **Raspberry layering** — supervisor crates are separate from core Fabro engine crates
8. **Direct trunk integration** — default autonomy model, PRs as optional escape hatch
9. **MiniMax for write, Opus for synthesis, Kimi for review** — provider policy centralized in `fabro-model/src/policy.rs`
10. **malinka/ package directory** — renamed from `fabro/` as the default output directory
