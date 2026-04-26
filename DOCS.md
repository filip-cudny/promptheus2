# Promptheus Tauri — Documentation Index

Top-level index of all `DOCS.md` files in the project. Updated incrementally as new areas are documented.

## Project Structure

- [src/DOCS.md](src/DOCS.md) — Svelte frontend: components, stores, services
- [src-tauri/DOCS.md](src-tauri/DOCS.md) — Rust backend: commands, services, models
- [src-tauri/src/models/DOCS.md](src-tauri/src/models/DOCS.md) — Data structures, serde conventions, settings schema
- [src-tauri/src/services/DOCS.md](src-tauri/src/services/DOCS.md) — Service layer: ClipboardService, ConfigService, error handling, lifecycle
- [src-tauri/src/services/mcp/DOCS.md](src-tauri/src/services/mcp/DOCS.md) — MCP client: rmcp wrapper, McpClient, McpError
- [src/lib/components/ui/DOCS.md](src/lib/components/ui/DOCS.md) — Reusable UI primitives: toast overlay, shared components
- [src/lib/components/context-menu/DOCS.md](src/lib/components/context-menu/DOCS.md) — Context menu popup window: sections, keyboard nav, item execution
- [src/lib/components/settings/DOCS.md](src/lib/components/settings/DOCS.md) — Settings dialog: sidebar nav, auto-save sections, models editor
- [docs/api-verification.md](docs/api-verification.md) — Process for verifying framework APIs against latest docs
- [docs/logging-levels.docs.md](docs/logging-levels.docs.md) — Log levels: when to use each with examples
- [docs/logging-sensitivity.docs.md](docs/logging-sensitivity.docs.md) — Data sensitivity tiers and per-module redaction rules
- [docs/logging-config.docs.md](docs/logging-config.docs.md) — Environment config, targets, rotation, performance
