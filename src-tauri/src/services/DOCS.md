# Services

Business logic layer. Services own state and behavior; Tauri commands delegate to them.

## Directory Structure

```
services/
├── mod.rs       # Module declarations
├── config.rs    # ConfigService — settings load/validate/save/mutate
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

### ConfigService specifics

**Load sequence**: `load_env()` -> read JSON -> deserialize -> `migrate_model_params()` -> `load_api_keys()` -> `validate()`.

**API key sanitization**: `save()` deep-clones settings before writing. Env-sourced model keys and speech model keys are stripped. Direct API keys are preserved.

**Mutation methods**: `add_model`, `update_model` (upsert), `delete_model`, `add_prompt`, `update_prompt`, `delete_prompt`, `reorder_prompts`, `update_notifications`, `update_speech_model`, `update_keymaps`, `update_menu_section_order`, `update_setting`.

### Testing pattern

Service tests use `tempfile::TempDir` to create isolated config directories. A `setup_test_dir()` helper copies the example settings into a temp dir. Tests that touch env vars should set and remove them within the test.

### Adding a new service

1. Create `services/<name>.rs` with error enum + struct + impl.
2. Add `pub mod <name>;` to `services/mod.rs`.
3. Wire into Tauri managed state in `lib.rs` (Task 4 pattern).
4. Add tests using `tempfile` for any file I/O.
