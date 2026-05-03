# Promptheus Tauri — Documentation Index

Top-level index of all `DOCS.md` files in the project. Updated incrementally as new areas are documented.

## Project Structure

- [src/DOCS.md](src/DOCS.md) — Svelte frontend: components, stores, services
- [src-tauri/DOCS.md](src-tauri/DOCS.md) — Rust backend: commands, services, models
- [src-tauri/src/models/DOCS.md](src-tauri/src/models/DOCS.md) — Data structures, serde conventions, settings schema
- [src-tauri/src/services/DOCS.md](src-tauri/src/services/DOCS.md) — Service layer: ClipboardService, ConfigService, error handling, lifecycle
- [src-tauri/src/services/mcp/DOCS.md](src-tauri/src/services/mcp/DOCS.md) — MCP client: rmcp wrapper, McpClient, McpError
- [src/lib/components/shared/ui/DOCS.md](src/lib/components/shared/ui/DOCS.md) — Dumb UI primitives: toast overlay, shared components
- [src/lib/components/features/context-menu/DOCS.md](src/lib/components/features/context-menu/DOCS.md) — Context menu popup window: sections, keyboard nav, item execution
- [src/lib/components/features/settings/DOCS.md](src/lib/components/features/settings/DOCS.md) — Settings dialog: sidebar nav, auto-save sections, models editor
- [src/lib/components/features/conversation-dialog/DOCS.md](src/lib/components/features/conversation-dialog/DOCS.md) — Chat window: composer, bubbles, sidebar, palette, tool calls
- [docs/api-verification.md](docs/api-verification.md) — Process for verifying framework APIs against latest docs
- [docs/logging-levels.docs.md](docs/logging-levels.docs.md) — Log levels: when to use each with examples
- [docs/logging-sensitivity.docs.md](docs/logging-sensitivity.docs.md) — Data sensitivity tiers and per-module redaction rules
- [docs/logging-config.docs.md](docs/logging-config.docs.md) — Environment config, targets, rotation, performance
- [docs/gotchas/tauri-command-threading.md](docs/gotchas/tauri-command-threading.md) — Sync Tauri commands run on GTK main thread; use `(async)` for blocking work
- [docs/gotchas/paste-handler.md](docs/gotchas/paste-handler.md) — Shift+Cmd/Ctrl+V raw paste: Mac uses arboard invoke, Linux uses native paste event
- [docs/gotchas/linux-gtk-focus.md](docs/gotchas/linux-gtk-focus.md) — `present_with_time` with real X11 timestamp from `x11_get_server_time`
- [docs/gotchas/linux-webkit-opacity.md](docs/gotchas/linux-webkit-opacity.md) — CSS opacity ignored on transparent WebKitGTK; use GTK `set_opacity()` instead
