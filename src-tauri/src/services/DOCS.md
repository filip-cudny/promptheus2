# Services

Business logic layer. Services own state and behavior; Tauri commands delegate to them.

## Directory Structure

```
services/
├── mod.rs               # Module declarations
├── ai/                  # AiService — multi-provider LLM completions (streaming + sync)
│   ├── mod.rs           #   AiService orchestrator, AiError enum
│   ├── provider.rs      #   AiProvider trait, CompletionRequest, StreamChunk
│   ├── openai.rs        #   OpenAiProvider — reqwest-based OpenAI implementation
│   └── sse.rs           #   Lightweight SSE line parser for reqwest byte streams
├── clipboard.rs         # ClipboardService — text and image clipboard operations
├── config.rs            # ConfigService — settings load/validate/save/mutate
├── menu_coordinator.rs  # MenuCoordinator — aggregates menu providers into ordered sections
├── context.rs           # ContextManagerService — ordered context items (text/image)
├── notification.rs      # NotificationService — event-gated Tauri event emission
├── placeholder.rs       # PlaceholderService — template variable substitution and image injection
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

### ContextManagerService specifics

Manages an ordered list of `ContextItem` values (text and/or images) in memory. Session-only — no persistence to disk.

**No internal locking**: the struct is a plain `Vec<ContextItem>`. Thread safety is provided by the `Mutex<AppState>` wrapper in the command layer, matching the project-wide pattern.

**No error enum**: all operations are infallible in-memory list manipulation.

**Constructor**: `ContextManagerService::new()` — returns an empty service.

**Text methods**: `set_context` (replace all), `append_context`, `get_context` (concatenates non-empty text with `\n`, returns `None` if no text), `has_context` (true if any Text items), `get_context_or_default`.

**Image methods**: `set_context_image` (replace all), `append_context_image`, `has_images`.

**General methods**: `clear`, `get_items` (cloned), `remove_item(index) -> bool`, `item_count`, `has_text_or_images`, `is_empty`.

**Key edge case**: `has_context()` checks for Text variant existence; `get_context()` additionally filters out empty-content items. Image-only context returns `None` from `get_context()`.

### NotificationService specifics

Holds an `AppHandle` to emit Tauri events to the frontend. Unlike other services, this one depends on the Tauri runtime — it uses the `Emitter` trait (`use tauri::Emitter`) to send events.

**Event gating**: `notify()` checks `NotificationSettings.events` before emitting. If the event is disabled in settings, the notification is silently dropped. Error-level notifications bypass the gate and always emit.

**Event name**: all notifications are emitted as a single `"notification"` Tauri event. The payload includes `level`, `title`, and optional `message`. The frontend listens on this one event name and routes by level.

**`is_event_enabled` mapping**: maps 12 string event names (e.g., `"prompt_execution_success"`, `"clipboard_copy"`) to the corresponding bool field on `NotificationEvents`. Unknown event names return `true` (safe default — show rather than hide).

**Methods**: `new(AppHandle)`, `notify(event_name, level, title, message, settings)`.

### MenuCoordinator specifics

Aggregates `MenuItemProvider` trait implementors (defined in `traits.rs`) into a flat, ordered list of `MenuItem`s. Section order comes from `ConfigService.settings().menu_section_order`.

**Built-in virtual sections**: `"prompts"` and `"settings"` are handled directly (not via providers). Prompts are built from config, with dynamic providers (ContextMenuProvider, LastInteractionMenuProvider, SpeechMenuProvider) excluded. Settings builds model and prompt toggle items.

**Provider sections**: Any other section ID queries registered providers whose `section_id()` matches. Providers are added at startup via `add_provider()`.

**Separator placement**: A `separator_after = true` flag is set on the last item of each non-final section. No trailing separator.

**Methods**: `new`, `add_provider`, `get_menu_items(&config)`, `refresh_all`.

**Testing**: Unit tests cover section ordering, separator placement, empty-provider skipping, dynamic-provider exclusion, and settings section content. Tests use mock providers implementing `MenuItemProvider`.

### PlaceholderService specifics

Resolves `{{name}}` template variables in prompt messages before they are sent to the LLM API. Also injects context images into the last message when present.

**Error variants**:
- `ClipboardUnavailable` — clipboard is empty or override is whitespace (propagates to caller)
- `ProcessorFailed` — non-clipboard processor error (logged, placeholder silently replaced with `""`)

**Processor trait**: `PlaceholderProcessor` defines `name`, `description`, and `process(context_override, clipboard, context_mgr)`. Two built-in processors are registered by `new()`:
- `ClipboardProcessor` — resolves `{{clipboard}}` from system clipboard or `context_override`
- `ContextProcessor` — resolves `{{context}}` from `ContextManagerService`

**Processing flow**:
1. `process_messages()` iterates messages as `(role, content)` pairs.
2. For each message, `process_content()` does single-pass string replacement for all registered `{{name}}` patterns.
3. For the last message, if `context_mgr.has_images()`, content is converted to `MessageContent::Parts` with text + image data URIs (`data:{media_type};base64,{data}`). Empty text is omitted from parts.
4. `ClipboardUnavailable` errors propagate immediately; other processor errors silently replace with `""`.

**Validation methods**: `has_placeholders()`, `find_invalid_placeholders()` (regex-based), `get_placeholder_info()`, `get_available_placeholders()`.

**Testing**: Tests use mock processors (registered via the trait) to avoid system clipboard dependency. `ContextManagerService` is used directly since it's in-memory. `ClipboardService` is passed but unused by mock processors.

### AiService specifics

Multi-provider LLM service. Lives in `ai/` subdirectory with one file per provider. Uses `reqwest` for HTTP (not provider-specific SDKs) so a single dependency serves all providers.

**Provider trait** (`provider.rs`): `AiProvider` defines `complete(CompletionRequest) -> Result<String>` and `complete_stream(CompletionRequest) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>>>>>`. Providers are framework-independent — no Tauri imports.

**Provider registry**: `AiService::new(models)` iterates `ModelConfig` entries, matches on the `Provider` enum, and creates the appropriate provider. Models with missing API keys or unsupported providers are tracked in `unavailable_models` (graceful degradation — the app still starts).

**Adding a new provider**:
1. Create `services/ai/<provider>.rs` implementing `AiProvider`.
2. Add a match arm in `AiService::new()` for the new `Provider` variant.
3. Add the variant to `Provider` enum in `models/settings.rs` and `types/index.ts`.

**Streaming architecture**: The `complete_stream` trait method returns an owned `Stream`. The Tauri command layer (`commands/ai.rs`) bridges this to a `tauri::ipc::Channel<StreamEvent>`. The `Mutex<AppState>` lock is released before stream iteration to avoid holding it for the duration of the HTTP response.

**SSE parser** (`sse.rs`): Converts a `reqwest::Response` byte stream into parsed `data:` line payloads. Handles line buffering, comment skipping, and `[DONE]` termination. Provider-agnostic — usable by any SSE-based provider.

**Error variants**: `ModelNotFound`, `ModelUnavailable`, `Authentication`, `Connection`, `RateLimit`, `ApiStatus { status, message }`, `Stream`, `Request`.

**OpenAiProvider** (`openai.rs`): Sends requests to `{base_url}/chat/completions`. Default base URL: `https://api.openai.com/v1`. Supports any OpenAI-compatible endpoint via the `base_url` config field. Model parameters (temperature, max_tokens, etc.) are merged into the request body when present.

### Adding a new service

1. Create `services/<name>.rs` with error enum + struct + impl.
2. Add `pub mod <name>;` to `services/mod.rs`.
3. Wire into Tauri managed state in `lib.rs` (Task 4 pattern).
4. Add tests using `tempfile` for any file I/O.
