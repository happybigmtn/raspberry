# Fabro Run Configuration Reference

A run config is the TOML file that launches a workflow graph. It bundles the
graph path with the execution settings needed for a repeatable run.

## Minimal Example

```toml
version = 1
graph = "workflow.fabro"
goal = "Implement the login feature"
```

Required fields:

- `version = 1`
- `graph = "..."`, resolved relative to the TOML file's directory

`goal` is optional. Goal precedence is:

- CLI `--goal`
- TOML `goal`
- graph `goal` attribute

## Common Sections

### Top level

- `version`
- `graph`
- `goal`
- `directory`

### `[llm]`

```toml
[llm]
model = "claude-sonnet-4-6"
provider = "anthropic"
```

Optional fallbacks:

```toml
[llm.fallbacks]
anthropic = ["gemini", "openai"]
```

### `[setup]`

```toml
[setup]
commands = ["bun install", "bun run build"]
timeout_ms = 120000
```

### `[sandbox]`

```toml
[sandbox]
provider = "local"
preserve = false
devcontainer = false
```

Common sub-sections:

- `[sandbox.local]`
- `[sandbox.daytona]`
- `[sandbox.daytona.snapshot]`
- `[sandbox.ssh]`
- `[sandbox.env]`

### `[vars]`

```toml
[vars]
repo_name = "myosu"
language = "rust"
```

These expand into the DOT source before parsing.

### `[checkpoint]`

```toml
[checkpoint]
exclude_globs = ["**/node_modules/**", "**/.cache/**"]
```

### `[assets]`

```toml
[assets]
include = ["artifacts/**", "screenshots/**"]
```

### `[pull_request]`

```toml
[pull_request]
enabled = true
draft = false
auto_merge = false
```

### `[github]`

```toml
[github]
permissions = { contents = "write" }
```

### `[[hooks]]`

```toml
[[hooks]]
event = "run_complete"
command = "echo done"
```

### `[mcp_servers]`

```toml
[mcp_servers.playwright]
type = "sandbox"
command = ["npx", "@playwright/mcp@latest", "--port", "3100", "--headless"]
port = 3100
```

## Resolution and Precedence

Important runtime facts:

- `graph` resolves relative to the TOML file, not the current shell directory
- unknown fields are rejected
- only `version = 1` is accepted

Useful precedence summary:

- node stylesheet or explicit node settings override broader defaults
- run config overrides project defaults from `fabro.toml`
- project defaults override server defaults from `~/.fabro/server.toml`

For model and provider specifically, the practical order is:

- CLI flags
- run config TOML
- project defaults
- server defaults
- graph attributes
- built-in defaults

## Validation

Use these commands:

```bash
fabro run run.toml --preflight
fabro validate workflow.fabro
```

`--preflight` is the best check when the workflow will be launched from TOML.

## Raspberry Notes

For a Raspberry-supervised lane, the run config should be boring and stable.
The supervisor mostly cares that the manifest can point to it and that Fabro
can resolve the graph predictably.

Good habits:

- keep the run config checked into the repo
- keep the graph path relative and local to the repo
- keep lane-specific setup and sandbox settings in the run config
- avoid hiding milestone semantics in TOML; milestone ownership belongs in the
  Raspberry manifest

If the user asks what a supervised repo must provide, answer that question from
the manifest and artifact contract first, then write the run config.
