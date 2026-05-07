# Backend (Rust + Tauri 2)

## Directory Structure

```
src-tauri/
├── tauri.conf.json             # Tauri config (windows, build, bundle, plugins)
├── Cargo.toml                  # Rust dependencies
├── build.rs                    # Tauri build script
├── capabilities/               # Permission capabilities for windows
│   └── default.json
├── icons/                      # App icons (generated via `pnpm tauri icon`)
└── src/
    ├── main.rs                 # 3-line shim — calls `app_lib::run()`
    ├── lib.rs                  # `pub fn run()` entry point: plugins, setup hook, invoke handler
    ├── error.rs                # Crate-wide `Error` enum + `Result<T>` alias
    ├── traits.rs               # Cross-module abstractions
    ├── setup/                  # App-init wiring, called from `lib.rs` setup hook
    │   ├── init.rs             # Top-level orchestrator
    │   ├── state.rs            # `app.manage(...)` calls — one per service
    │   ├── windows.rs          # Window/webview creation
    │   ├── tray.rs / menu.rs   # System tray + native menu
    │   ├── shortcuts.rs        # Global hotkey registration
    │   ├── background.rs       # `tauri::async_runtime::spawn` background tasks
    │   └── log.rs              # Logging plugin config
    ├── commands/               # IPC surface — thin adapters, no business logic
    │   ├── mod.rs              # `pub mod` re-exports + `handlers!` macro
    │   ├── ai.rs / ai_webview.rs
    │   ├── execution_stream.rs / execution_control.rs / execution_generation.rs
    │   ├── prompts.rs / skills.rs / mcp.rs
    │   ├── settings.rs / settings_dialog.rs
    │   ├── history.rs / history_dialog.rs
    │   ├── context.rs / context_editor.rs
    │   ├── menu.rs / provider_menu.rs / dock.rs
    │   ├── conversation_dialog.rs / image_preview.rs / text_preview.rs
    │   ├── notification.rs / clipboard.rs / speech.rs
    │   └── tokenizer.rs / ui_state.rs
    ├── services/               # Business logic — no Tauri imports, unit-testable
    │   ├── ai/                 # Provider trait + per-provider implementations
    │   ├── ai_webview/         # AI webview lifecycle, cold-suspend
    │   ├── config/             # Settings load/save, hot-reload, migrator
    │   ├── execution/          # Skill executor, streaming, tool-call orchestration
    │   ├── mcp/                # MCP client, server registry
    │   ├── skill/              # Skill loader + parser
    │   ├── speech/             # Audio capture + transcription
    │   ├── sqlite_history/     # History persistence + codec
    │   ├── frontmost_app/      # Per-OS frontmost-app detection
    │   ├── menu_coordinator.rs # Aggregates menu providers into ordered sections
    │   └── …                   # placeholder, notification, clipboard, hotkeys, etc.
    ├── models/                 # Serde types crossing the IPC boundary
    └── providers/              # Menu item generators implementing `MenuItemProvider`
```

## Conventions

### Entry points: `lib.rs` is the app, `main.rs` is a stub

`main.rs` is a 3-line shim (`fn main() { app_lib::run() }`) — mobile builds compile the crate as a library and never see it. All real wiring lives in `lib.rs::run()` (`#[cfg_attr(mobile, tauri::mobile_entry_point)]`), which delegates to the `setup/` module so the `Builder` chain stays under ~50 lines.

When adding plugins, state, or background tasks: extend the right `setup/` submodule, not `lib.rs` itself.

### Commands vs Services

- **Commands** (`commands/`) — thin Tauri `#[tauri::command]` handlers. Deserialize args, call a service, return `crate::Result<T>`. No business logic.
- **Services** (`services/`) — pure business logic. No Tauri imports. Unit-testable in isolation.

```rust
// commands/prompts.rs
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;
use crate::{models::ExecutionResult, services::PromptService, Result};

#[tauri::command]
pub async fn execute_prompt(
    svc: State<'_, Arc<Mutex<PromptService>>>,
    prompt_id: String,
    input: String,
) -> Result<ExecutionResult> {
    svc.lock().await.execute(&prompt_id, &input).await
}
```

#### Command body discipline

A command file deserializes input, looks up state, calls a service, returns the result. Inside `commands/` do **not**:

- Define module-level `static` state (move it to a service struct managed in `setup::state`).
- Implement business helpers (move to `services/`).
- Build response payloads with inline `serde_json::json!` (define a `models/` struct or extend an existing one).

If a command file grows past ~300 LOC or starts holding its own state machine, the logic belongs in a service. Current offenders worth refactoring as we touch them: `commands/conversation_dialog.rs`, `commands/speech.rs`, `commands/menu.rs`.

#### Splitting a module

Split when one of these is true:

1. A file passes ~300–400 LOC and contains more than one cohesive concern.
2. Two unrelated `State<T>` flow through the same file (one module per service).
3. A subsystem has its own lifecycle (background task, watcher, plugin-like surface) — it earns its own module under `services/<name>/` with `mod.rs` exporting an `init()`/`shutdown()` (or constructor + `Drop`).
4. Tests are getting awkward — if mocking one piece requires touching unrelated code, the seam is wrong.

Don't pre-split. Three short files with one function each is worse than one 200-line file. Wait for the friction.

### Registering commands: the `handlers!` macro

Commands are registered through the `handlers!` macro defined in `commands/mod.rs`. `tauri::generate_handler!` only accepts a flat token list — concatenation must happen at macro expansion time, not via `Vec` — so we wrap the full list in `macro_rules! handlers!` and call `crate::handlers!()` from `lib.rs`. Group entries per source file with a `// === <file> ===` comment.

When adding a command: add the `pub mod` line at the top of `commands/mod.rs`, then a `$crate::commands::<file>::<fn>` entry in the matching `// === <file> ===` block of the macro.

### State Management

- One struct per concern. We **do not** use a single God-state — each service is managed individually as `Arc<Mutex<…Service>>` (or `Arc<…>` if internally synchronized) by `setup::state`.
- Commands take only the services they need: `State<'_, Arc<Mutex<ConfigService>>>`, not a monolithic handle.
- Initialize state inside `setup::init::run` (called from `Builder::setup`) when construction needs `AppHandle` (e.g. resolving `app.path()` directories). Plain values can be `manage`d before `.setup()`.

#### Mutex choice

Pick the right primitive — getting this wrong deadlocks or freezes the app:

- **Service state held across `.await` → `tokio::sync::Mutex`.** This is the default for anything reachable from an async command.
- **`std::sync::Mutex` is allowed only for short, await-free guards** — typical examples are command-local statics that swap an `Option`, e.g. `commands/notification.rs`, `commands/text_preview.rs`, `commands/conversation_dialog.rs`.
- **When a `std::sync::Mutex` lives on a service struct** (e.g. `services/ai_webview` snapshot maps), every guard **must drop before any `.await`**. Holding a sync mutex across `.await` deadlocks the runtime; holding it across a streamed response stalls every other command.

Cross-reference: see `docs/gotchas/tauri-command-threading.md` for the related GTK-thread issue on Linux sync commands.

### Errors

One crate-level enum `crate::Error` in `error.rs`, one alias `crate::Result<T>`. All `#[tauri::command]`s return `crate::Result<T>` — `String` errors and `anyhow::Error` are not allowed at the IPC boundary (`anyhow` does not implement `Serialize`).

The enum is **layered**, not a single flat list:

- Each service owns its own `*Error` thiserror enum (`AiError`, `ConfigError`, `ExecutionError`, `McpError`, `SkillError`, `HistoryError`, …).
- `crate::Error` wires every service error in via `#[from]`, so commands can use `?` directly.
- Foreign error types we cross often (`std::io::Error`, `serde_json::Error`, `rusqlite::Error`, `tauri::Error`) are also `#[from]` on `crate::Error`.
- `crate::Error::Other(String)` is the explicit escape hatch for one-off cases that don't justify a new variant. `From<String>`/`From<&str>` are implemented for it.

Serialization is hand-rolled and flattens to a string (`s.serialize_str(&self.to_string())`) — the frontend treats errors as opaque user-facing messages. If a feature needs the frontend to discriminate variants, extend `Serialize` to emit `{ kind, message }` once for the whole enum, don't do it per-call site.

`anyhow` is fine **inside** services for ergonomic `?` chaining; convert at the service's public boundary into the service-level error.

### IPC channels

Pick the right channel for each direction:

| Need                                              | Use                                |
| ------------------------------------------------- | ---------------------------------- |
| Request/response, command-style                   | `#[tauri::command]` + `invoke`     |
| Backend → frontend stream tied to a single call   | `tauri::ipc::Channel<T>`           |
| Broadcast to all listeners (lifecycle, app-wide)  | `app.emit("name", payload)`        |
| Window-scoped notification                        | `app.emit_to(label, "name", …)`    |
| Frontend → backend without return value           | Still use `invoke` — events from JS to Rust are harder to type and harder to permission |

Default to commands. Reach for `app.emit*` only for true one-to-many notifications. Streamed results (AI completions, skill execution, generation events) belong on `Channel<T>` — typed, scoped to the invoke call, and back-pressure-friendly. The reference implementation is `commands/ai.rs` + `commands/execution_stream.rs`.

**Do not pair `Channel<T>` with `app.emit` for the same call.** Pick one. The Channel is the source of truth for call-scoped events; layering a global broadcast on top means subscribers can't tell which run finished, and consumers end up de-duplicating in two places. (See `services/execution/skill_executor.rs` and `commands/speech.rs` — both have legacy duplicate-broadcast paths queued for cleanup.)

### Async

- Tauri ships with `tokio` via `tauri::async_runtime`. Use it; don't pull a second runtime.
- `#[tauri::command] async fn` runs on a tokio task. `State<'_, T>` requires the explicit lifetime — that's not optional in async commands.
- Long-running background work (watchers, pollers): spawn with `tauri::async_runtime::spawn` from `setup::background`, not from inside a command.
- When a command both holds a `Mutex<Service>` lock and iterates a long-lived stream, clone the cheap inner handle (typically `Arc<Provider>`), drop the guard, then iterate. Keeps the service responsive. See `commands/ai.rs` for the pattern.

### Models

- All structs crossing the IPC boundary derive `serde::Serialize` and/or `serde::Deserialize`.
- Rust uses `snake_case`, JS uses `camelCase` — `#[serde(rename_all = "camelCase")]` on the struct handles conversion.

### Provider Pattern

Menu content is assembled from pluggable providers (`providers/`):

- Each provider implements `MenuItemProvider` and returns `Vec<MenuItem>`.
- Providers are registered at startup and queried when the menu is shown.
- Adding a new menu section = adding a new provider.

### Logging

Uses `tauri-plugin-log` with the standard Rust `log` crate. Logs go to stdout, a log file (in the app data directory), and the webview console.

- Rust: use `log::{info, warn, error, debug}` macros directly.
- Frontend: `import { error, info, warn } from "@tauri-apps/plugin-log"`.
- Both `main.ts` and `context-menu-main.ts` call `attachConsole()` to bridge Rust logs into browser devtools.
- Default level is `Info` globally, `Debug` for the app crate. Override with `RUST_LOG` env var.
- Log at decision points and errors, not at every function boundary.

Detailed guides (load only what you need):

- [Log levels reference](../docs/logging-levels.docs.md) — when to use each level with examples
- [Data sensitivity](../docs/logging-sensitivity.docs.md) — what to log, redact, or never log per module
- [Configuration & rotation](../docs/logging-config.docs.md) — env-specific levels, targets, file rotation, performance

### Hotkey Actions

Global hotkeys are handled **entirely in the Rust backend** (`setup/shortcuts.rs` + `services/hotkeys.rs`). Do **not** route hotkey actions through `app.emit()` to the frontend — this app has no persistent main window, so frontend event listeners are unreliable.

When adding a new hotkey action:

1. Add the action string to `services/hotkeys.rs` (binding resolution).
2. Add a match arm in the action dispatcher.
3. Implement the action as a Rust async function or call an existing command directly.

### App Lifecycle

This is a **tray-only app** — no main window opens on startup. On macOS, activation policy is set to `Accessory` (no Dock icon, no Cmd+Tab entry). UI is shown on demand via:

- System tray menu (native)
- Context menu window (borderless Tauri window, hotkey-triggered)
- Prompt dialog windows (created dynamically)

When showing a window from a background context (e.g., global hotkey), call `app.show()` on macOS before `win.show()` to activate the app and ensure the window appears in front.

Because the app is tray-only with multiple short-lived windows, cross-window coordination via `app.emit_to(label, …)` is legitimate (e.g. `prompts.rs`, `provider_menu.rs`, `conversation_dialog.rs`). That is **not** the events-as-substitute-for-commands anti-pattern — there is no single window from which to drive a command. Within a single window, prefer `invoke`.

### Capabilities

- `capabilities/default.json` declares permissions for the app's webviews.
- Our own `#[tauri::command]`s do **not** need a capability entry — those checks apply only to plugin APIs and the core API surface exposed to the webview.
- When adding a Tauri plugin, add its permissions here (e.g., `"clipboard-manager:default"`).
- **Always scope `fs:*` permissions with explicit paths.** Never grant a bare `fs:allow-write-text-file` — narrow scopes (`$APPCONFIG/*`, `$APPDATA/promptheus/**`) are mandatory, otherwise any window can write any file the user can.
- Split `default.json` per feature once it grows past ~50 lines or two windows have non-overlapping needs (e.g. extract `settings.json` for the settings dialog so the context-menu window doesn't inherit settings-only permissions).

### Configuration

- `tauri.conf.json` is the single source of truth. Use `tauri.conf.<platform>.json` for platform overrides rather than runtime branching.
- Bundle identifier `com.promptheus.desktop` (reverse-domain). Drives macOS `~/Library/Application Support/<id>/` and Linux `~/.config/<id>/`. Renaming after release is painful — don't.
- App-runtime configuration (user settings) is **not** stored via `tauri-plugin-store`. We use `services/config/` instead because the settings layer needs hot-reload on file change, schema migration on version bumps, and composition over multiple defaults files — capabilities the plugin doesn't offer. If you find yourself reaching for `tauri-plugin-store` for new settings, extend `services/config/` instead.

#### Path resolution in settings

Two contracts, one shared resolver. Picking the wrong one for a new field will silently break portability or open path-traversal holes — match the data shape, don't invent a third rule.

- **Config-relative** (in-app data files the user manages alongside `settings.json`): resolved through `services/config/path.rs::resolve_config_relative`, which expands `${VAR}` references then enforces non-empty + non-absolute + no-`..` and joins to `app_config_dir`. Current fields: `prompt_base.*`, `surfaces.title_generation.prompt`, `surfaces.speech_to_text.prompt`, `surfaces.speech_to_text.keyterms_file`. `PromptStore::resolve` wraps this and additionally requires `.md`/`.markdown`. Add new data-file fields by going through this helper, not `config_dir.join(raw)` — the latter silently accepts absolute paths because of `PathBuf::join` semantics.
- **Absolute / system-relative** (pointers to executables and dirs outside the app): MCP `command`, `args`, and `env` values. Resolved through `services/env_resolve::resolve_env_refs` only — no joining, no validation. The user owns absolute correctness. `command` additionally falls through `services/shell_env::resolve_command`, which does PATH lookup for bare names; `args` and env values do not.

Placeholders supported in any field that goes through `resolve_env_refs` (which is both contracts):

- `${VAR}` — any process env var (loaded from the user's shell + `<config_dir>/.env`).
- `${CONFIG_DIR}` — the resolved `app_config_dir`. Set by `services/config/loader::load_env` after dotenv processing, so it is reliable and not user-overridable from `.env`. Useful in MCP `args`/`env` to point at config-bundled data without hardcoding `/Users/...`. **Has no effect in config-relative fields** — `${CONFIG_DIR}/foo.md` expands to an absolute path and is rejected; write `foo.md` instead.
- `${HOME}` — provided by the OS on Unix. Not synthesized by the app, so on Windows callers get whatever the environment provides.

Cwd is **not** part of either contract. Tauri inherits the launch cwd (Finder/launchd give `/`); MCP child processes inherit ours. Never write a relative path expecting `cwd`-relative resolution — there is no consistent cwd to be relative to.

### Dependency hygiene

- `tauri` is pinned to an exact patch version — breaking changes happen at minor releases, and 2.x patches occasionally tighten internals our code touches via `unstable`/`macos-private-api` features.
- `tauri-plugin-*` crates are pinned exactly **but independently** of Tauri core. The official plugin ecosystem does not version in lockstep with core minors (no `2.10.x` line of plugins exists). The pins in `Cargo.toml` are the latest releases known to build against the core version we use; bump them deliberately, not in bulk.
- `serde`, `tokio`, `thiserror` track latest stable major lines — they're pulled in transitively anyway, divergence costs duplicate compilation.
- Before adding any dependency, verify the current latest version (`cargo search`); follow the verification process in `docs/api-verification.md`.

### Testing

- Services are plain Rust structs with no Tauri dependency — unit-test them like any library. ~30 service files have `#[cfg(test)]` blocks; that's the expected pattern.
- Commands themselves are too thin to test directly. If a command holds enough logic to warrant a test, that logic belongs in a service.
- For integration tests that need an `AppHandle`, use `tauri::test::mock_app()` — adopt only when a specific test demands it; we currently don't ship integration tests.
