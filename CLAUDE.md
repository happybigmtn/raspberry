# Arc

## Dev servers

1. `arc serve` — starts the Rust API server (demo mode is per-request via `X-Arc-Demo: 1` header)
2. `cd apps/arc-web && bun run dev` — starts the React dev server

## API workflow

The OpenAPI spec at `docs/api-reference/arc-api.yaml` is the source of truth for the arc-api HTTP interface.

1. Edit `docs/api-reference/arc-api.yaml`
2. `cargo build -p arc-types` — build.rs regenerates Rust types via typify
3. Write/update handler in `crates/arc-api/src/server.rs`, add route to `build_router()`
4. `cargo test -p arc-api` — conformance test catches spec/router drift
5. `cd packages/arc-api-client && bun run generate` — regenerates TypeScript Axios client

## Shell quoting in sandbox code

When interpolating values into shell command strings (in `arc-exe` and `arc-workflows`), always use the `shell_quote()` helper (backed by `shlex::try_quote`). Never use manual `replace('\'', "'\\''")` or unquoted interpolation. This applies to file paths, branch names, URLs, env vars, image names, glob patterns, and any other user-controlled input assembled into a shell script.

## Testing workflows

When manually testing workflows with `arc run`, use `--no-retro` to skip the retro step and finish faster.
