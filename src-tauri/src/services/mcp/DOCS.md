# MCP Service

MCP (Model Context Protocol) client module. Thin wrapper around the `rmcp` crate (official Rust MCP SDK) for spawning and communicating with MCP tool servers over stdio.

## Files

```
mcp/
├── mod.rs       # Module declarations, re-exports McpClient, McpError
├── client.rs    # McpClient struct, McpError enum
└── DOCS.md
```

## McpClient

Wraps `rmcp::RunningService<RoleClient, ()>`. Each instance manages one MCP server child process.

**Lifecycle**: `start()` spawns the process and completes the MCP initialize handshake (handled by rmcp). `shutdown()` drops the service, closing the transport and signaling the server to exit.

**Methods**:
- `start(server_name, command, args, env)` — spawn server process, return connected client
- `list_tools()` — paginated tool listing via `peer().list_all_tools()`
- `call_tool(name, arguments)` — execute tool with 30s timeout
- `shutdown(self)` — consume client, drop service

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
