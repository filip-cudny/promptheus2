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
    ├── main.rs                 # Entry point (calls lib::run)
    ├── lib.rs                  # App builder setup, plugin registration
    ├── commands/               # Tauri command handlers (thin wrappers)
    │   ├── mod.rs
    │   ├── clipboard.rs        # Clipboard read/write commands
    │   ├── menu.rs             # Context menu commands (get items, execute, show window)
    │   ├── prompt.rs           # Prompt execution commands
    │   ├── settings.rs         # Config read/write commands
    │   ├── history.rs          # History CRUD commands
    │   ├── context.rs          # Context management commands
    │   └── system.rs           # Clipboard, notifications, speech
    ├── services/               # Business logic (no Tauri dependency)
    │   ├── mod.rs
    │   ├── openai.rs           # OpenAI API client (multi-model)
    │   ├── config.rs           # Settings load/save, hot-reload
    │   ├── history.rs          # History storage
    │   ├── context.rs          # Context manager (text + images)
    │   ├── clipboard.rs        # System clipboard access
    │   ├── hotkey.rs           # Global hotkey registration
    │   ├── menu_coordinator.rs # Aggregates menu providers into ordered sections
    │   ├── notification.rs     # Desktop notifications
    │   ├── speech.rs           # Audio recording + transcription
    │   └── placeholder.rs      # Template variable substitution
    ├── models/                 # Data structures (serde Serialize/Deserialize)
    │   ├── mod.rs
    │   ├── menu.rs             # MenuItem, MenuItemType
    │   ├── prompt.rs           # PromptData, PromptMessage
    │   ├── execution.rs        # ExecutionResult, ErrorCode
    │   ├── history.rs          # HistoryEntry, ConversationData
    │   ├── context.rs          # ContextItem (text/image)
    │   └── settings.rs         # Full settings structure
    ├── traits.rs               # Shared traits (MenuItemProvider)
    └── providers/              # Menu item generators
        ├── mod.rs
        ├── prompt_provider.rs
        ├── history_provider.rs
        ├── context_provider.rs
        ├── speech_provider.rs
        └── system_provider.rs
```

## Conventions

### Commands vs Services

- **Commands** (`commands/`) — thin Tauri `#[tauri::command]` handlers. Extract args, call a service, return result. No business logic here.
- **Services** (`services/`) — pure business logic. No Tauri imports. Testable independently.

```rust
// commands/prompt.rs
#[tauri::command]
pub async fn execute_prompt(
    state: State<'_, Mutex<AppState>>,
    prompt_id: String,
    input: String,
) -> Result<ExecutionResult, String> {
    let state = state.lock().await;
    state.prompt_service.execute(&prompt_id, &input).await.map_err(|e| e.to_string())
}
```

### State Management

- App state lives in `Mutex<AppState>` managed by Tauri's `Manager` API.
- Use `tokio::sync::Mutex` for async commands (not `std::sync::Mutex`).
- Register in `lib.rs` setup: `app.manage(Mutex::new(AppState::default()))`.
- Access in commands via `state: State<'_, Mutex<AppState>>`.

### Models

- All structs exposed to frontend must derive `serde::Serialize` and/or `serde::Deserialize`.
- Rust uses `snake_case`, JS uses `camelCase` — serde handles conversion with `#[serde(rename_all = "camelCase")]`.

### Provider Pattern

Menu content is assembled from pluggable providers:

- Each provider implements a trait that returns `Vec<MenuItem>`.
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

Global hotkeys are handled **entirely in the Rust backend** (`lib.rs → execute_hotkey_action`). Do **not** route hotkey actions through `app.emit()` to the frontend — this app has no persistent main window, so frontend event listeners are unreliable.

When adding a new hotkey action:

1. Add the action string to `services/hotkeys.rs` (binding resolution).
2. Add a match arm in `execute_hotkey_action()` in `lib.rs`.
3. Implement the action as a Rust async function or call an existing command directly.

### App Lifecycle

This is a **tray-only app** — no main window opens on startup. On macOS, activation policy is set to `Accessory` (no Dock icon, no Cmd+Tab entry). UI is shown on demand via:

- System tray menu (native)
- Context menu window (borderless Tauri window, hotkey-triggered)
- Prompt dialog windows (created dynamically)

When showing a window from a background context (e.g., global hotkey), call `app.show()` on macOS before `win.show()` to activate the app and ensure the window appears in front.

### Capabilities

- `capabilities/default.json` declares permissions for app windows.
- When adding a Tauri plugin, add its permissions here (e.g., `"clipboard-manager:default"`).
