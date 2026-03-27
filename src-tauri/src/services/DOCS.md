# Services

Business logic layer. Services own state and behavior; Tauri commands delegate to them.

## Directory Structure

```
services/
├── mod.rs            # Module declarations
├── clipboard.rs      # ClipboardService — text and image clipboard operations
├── config.rs         # ConfigService — settings load/validate/save/mutate
├── notification.rs   # NotificationService — event-gated Tauri event emission
└── DOCS.md
```

## Conventions

### Error handling

Each service defines its own error enum using `thiserror::Error`. Variants use `#[from]` for automatic conversion from upstream errors (`std::io::Error`, `serde_json::Error`).

```rust
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    // ...
}
```

### Service lifecycle

Services are plain structs (not singletons). They are created once at startup and placed into Tauri managed state behind `Mutex`. The command layer is responsible for locking and calling service methods.

- **Constructor**: `Service::load(args) -> Result<Self, ServiceError>` — reads from disk, validates, returns ready-to-use instance.
- **Persistence**: `save(&self)` writes to disk. Mutation methods do **not** auto-save — the command layer decides when to persist.
- **Reload**: `reload(&mut self)` re-reads from disk, replacing in-memory state.

### ClipboardService specifics

Uses the `arboard` crate for cross-platform clipboard access (text and images). No Tauri dependency — keeps the service layer framework-independent.

**Key pattern — no stored clipboard handle**: `arboard::Clipboard` is not `Send`/`Sync`, so it cannot be held across await points or stored in shared state. Each method creates a fresh `arboard::Clipboard` instance per call. The `ClipboardService` struct itself is a unit struct.

**Error variants**:
- `Unavailable` — clipboard is empty or content can't be read (soft error)
- `Access` — clipboard system is inaccessible (hard error)
- `ImageConversion` — image encode/decode failed

**Image pipeline**: `arboard::ImageData` (raw RGBA pixels) → `image::ImageBuffer` → PNG encode → base64 string. Returns `(base64_data, media_type)` tuple.

**Methods**: `new`, `get_text`, `set_text`, `is_empty`, `has_image`, `get_image_base64`.

### ConfigService specifics

**Load sequence**: `load_env()` -> read JSON -> deserialize -> `migrate_model_params()` -> `load_api_keys()` -> `validate()`.

**API key sanitization**: `save()` deep-clones settings before writing. Env-sourced model keys and speech model keys are stripped. Direct API keys are preserved.

**Mutation methods**: `add_model`, `update_model` (upsert), `delete_model`, `add_prompt`, `update_prompt`, `delete_prompt`, `reorder_prompts`, `update_notifications`, `update_speech_model`, `update_keymaps`, `update_menu_section_order`, `update_setting`.

### Testing pattern

Service tests use `tempfile::TempDir` to create isolated config directories. A `setup_test_dir()` helper copies the example settings into a temp dir. Tests that touch env vars should set and remove them within the test.

### NotificationService specifics

Holds an `AppHandle` to emit Tauri events to the frontend. Unlike other services, this one depends on the Tauri runtime — it uses the `Emitter` trait (`use tauri::Emitter`) to send events.

**Event gating**: `notify()` checks `NotificationSettings.events` before emitting. If the event is disabled in settings, the notification is silently dropped. Error-level notifications bypass the gate and always emit.

**Event name**: all notifications are emitted as a single `"notification"` Tauri event. The payload includes `level`, `title`, and optional `message`. The frontend listens on this one event name and routes by level.

**`is_event_enabled` mapping**: maps 12 string event names (e.g., `"prompt_execution_success"`, `"clipboard_copy"`) to the corresponding bool field on `NotificationEvents`. Unknown event names return `true` (safe default — show rather than hide).

**Methods**: `new(AppHandle)`, `notify(event_name, level, title, message, settings)`.

### Adding a new service

1. Create `services/<name>.rs` with error enum + struct + impl.
2. Add `pub mod <name>;` to `services/mod.rs`.
3. Wire into Tauri managed state in `lib.rs` (Task 4 pattern).
4. Add tests using `tempfile` for any file I/O.
