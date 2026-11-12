# MCP Service

MCP (Model Context Protocol) client module. Thin wrapper around the `rmcp` crate (official Rust MCP SDK) for spawning and communicating with MCP tool servers over stdio.

`mod.rs` re-exports `McpError` and `McpRegistry`; `McpClient` is reached as `client::McpClient`. `McpRegistry` (`registry.rs`) starts every configured server (`start_all`), indexes tools by qualified name `<server>.<tool>` (a duplicate is skipped with a `warn` log) and routes `call_tool` to the owning server.

**Server states**: a server that fails to start is dropped from `servers`, but `start_all` records the error's `Display` text (no env values, no resolved secrets) in `start_errors`. `server_states()` returns one `McpServerState` per server the registry tried (`name`, `state` `running` | `error` | `off`, `error`, `tools` = raw tool names), sorted by name. `off` is a server whose `disabled` flag was true at this start: `start_all` does not start it (one `info` line), it has no `error` and no tools, and `get_tools_by_qualified_names` returns nothing for a qualified name of it without the `warn` an unknown name gets (the server part is matched exactly, `<server>.`). The command `list_mcp_servers` returns it; the registry is built once at startup, so the states reflect that start, not later config edits.

**Switched-off tools**: `McpServerConfig.disabled_tools` (raw tool names) is read from the current config at each use, so it applies at once, unlike server changes. `registry::without_disabled_tools(qualified_names, servers)` drops every `<server>.<tool>` listed there (exact match, order kept); the chat (`ConversationExecutor::resolve_tools`, also behind token counting in `commands/tokenizer.rs`) and the settings agent (`engine.rs`, before `select_enabled`) call it before resolving names, so a switched-off tool is sent to no model and its `enabled_tools` entries stay untouched. `list_mcp_tools` still lists every registry tool, with `disabled: true` for a switched-off one, so the server editor can show and re-enable it.

The registry has two callers: the chat (`ConversationExecutor`, tools from the chat's `enabled_tools`) and the settings agent (`settings_agent::mcp::RegistrySource`, tools from `surfaces.settings_agent.generation.enabled_tools`; see `services/settings_agent/DOCS.md`). Both call through `execution::stream::call_mcp_tool` and send tools under their raw name; the agent drops a raw name that clashes (with a settings tool or another enabled tool) instead of resolving it through `resolve_raw_name`. The agent never starts or restarts servers.

## McpClient

Wraps `rmcp::RunningService<RoleClient, ()>`. Each instance manages one MCP server child process. Tool calls go through a cloned `Peer`; the service sits behind a `tokio::sync::Mutex` that only `close` takes, so a running tool call never blocks closing.

**Lifecycle**: `start(server_name, command, args, env, timeout_secs, tool_timeouts)` spawns the process (with the login-shell environment from `shell_env`) and completes the MCP initialize handshake (handled by rmcp). `close(&self, timeout)` calls `RunningService::close_with_timeout`, which cancels the service and waits for the transport to close (graceful shutdown of the child process); it logs `info` when closed and `warn` on timeout or error.

**Timeouts**: `call_tool` uses the per-tool entry of `tool_timeouts`, else `timeout_secs` (the `McpServerConfig` default is 180 s). Expiry returns `McpError::Timeout`.

## Closing on exit

`lib.rs` handles `RunEvent::Exit` by blocking (`tauri::async_runtime::block_on`) on `McpRegistry::close_all(&self, timeout)`, which closes all servers concurrently with a timeout of 4 s, so the whole close takes at most about 4 s. The timeout must exceed rmcp 1.4.0's `MAX_WAIT_ON_DROP_SECS = 3` (`transport/child_process.rs`): after that wait `graceful_shutdown` kills a child that did not exit on its own, and a shorter timeout returns first, the process exits and the kill never runs. Dropping the registry is not enough: Tauri ends the process with `std::process::exit`, so the managed `Arc<McpRegistry>` is never dropped, and rmcp's drop of the child only spawns the kill task, which does not get to run.

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
