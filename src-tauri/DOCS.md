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

Mirrors the original app's architecture:

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

### Capabilities

- `capabilities/default.json` declares permissions for all app windows (`main` and `context-menu`).
- When adding a Tauri plugin, add its permissions here (e.g., `"clipboard-manager:default"`).
