Contract written to `.fabro-work/contract.md`.

**Summary:**

Identified gaps in existing test coverage:

| Crate | Function | Gap |
|-------|----------|-----|
| fabro-mcp | `McpClient::new` | No tests for sandbox transport error |
| fabro-mcp | `LoggingClientHandler` | No tests for notification handlers |
| fabro-github | `embed_token_in_url` | 0 tests |
| fabro-github | `create_pull_request` | 0 tests |
| fabro-github | `enable_auto_merge` | 0 tests |
| fabro-github | `resolve_clone_credentials` | 0 tests |

**Planned additions:**
- 4 new tests in `fabro-mcp` (client_handler + client error path)
- 6+ new tests in `fabro-github` (embed_token, create_pr, enable_auto_merge, resolve_clone_credentials)
- 1 optional new test file for HTTP transport (may skip if mocking impractical)