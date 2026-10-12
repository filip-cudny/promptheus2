# MCP Service

MCP (Model Context Protocol) client module. Thin wrapper around the `rmcp` crate (official Rust MCP SDK) for spawning and communicating with MCP tool servers over stdio.

`mod.rs` re-exports `McpError` and `McpRegistry`; `McpClient` is reached as `client::McpClient`. `McpRegistry` (`registry.rs`) starts every configured server (`start_all`), indexes tools by qualified name `<server>.<tool>` (a duplicate is skipped with a `warn` log) and routes `call_tool` to the owning server.

## McpClient

Wraps `rmcp::RunningService<RoleClient, ()>`. Each instance manages one MCP server child process.

**Lifecycle**: `start(server_name, command, args, env, timeout_secs, tool_timeouts)` spawns the process (with the login-shell environment from `shell_env`) and completes the MCP initialize handshake (handled by rmcp). `shutdown(self)` drops the service, closing the transport and signaling the server to exit.

**Timeouts**: `call_tool` uses the per-tool entry of `tool_timeouts`, else `timeout_secs` (the `McpServerConfig` default is 180 s). Expiry returns `McpError::Timeout`.

## McpError

Four variants: `StartFailed`, `Protocol`, `Timeout`, `Service`. All use `thiserror::Error`.

## Key types from rmcp

Protocol types come from `rmcp::model` — no custom type wrappers:
- `rmcp::model::Tool` — tool definition (name, description, inputSchema)
- `rmcp::model::CallToolResult` — tool execution result (content, isError)
- `rmcp::model::CallToolRequestParams` — constructed via `::new(name).with_arguments(map)`
- `rmcp::model::Content` — content item enum (Text, Image, Resource)

## Conventions

- `Value::Object` arguments pass through directly; non-object values are wrapped as `{"value": ...}`
- `CallToolRequestParams` is non-exhaustive — always use the builder pattern (`::new().with_arguments()`)
- `TokioChildProcess::new(cmd)` is the entry point for spawning (not `TokioChildProcessBuilder::new`, which is private)
