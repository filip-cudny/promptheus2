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
├── ai_webview/          # AiWebviewState + provider window management
│   ├── mod.rs           #   AiWebviewState (shared mutex maps), public API re-exports
│   ├── lifecycle.rs     #   Window/webview creation, teardown, media permissions
│   ├── provider_swap.rs #   Hosted/standalone provider swap, active-changed events
│   ├── palette.rs       #   Palette open/close + router-message handling
│   ├── cold_suspend.rs  #   Idle tracking, suspend-to-blank, lifecycle toasts
│   └── scripts.rs       #   Initialization JS (dark-mode shim + palette keybind)
├── clipboard.rs         # ClipboardService — text and image clipboard operations
├── config/              # ConfigService — settings load/validate/save/mutate
│   ├── mod.rs           #   ConfigService struct + load/save/reload + mutators; SurfaceKind
│   ├── loader.rs        #   File I/O, env-var loading, default-asset initialisation
│   ├── migrator.rs      #   Legacy schema migration (default_model → surfaces, inline → file)
│   ├── prompts.rs       #   PromptKind enum + PromptStore (file I/O + path safety)
│   ├── defaults.rs      #   Surface-default backfill + Settings validation
│   └── tests.rs         #   Integration tests using ConfigService::load with TempDir
├── context.rs           # ContextManagerService — ordered context items (text/image)
├── database.rs          # Database — SQLite connection, schema creation, migrations
├── sqlite_history/      # SqliteHistoryService — persistent history storage via SQLite
│   ├── mod.rs           #   SqliteHistoryService CRUD + public API
│   ├── codec.rs         #   TreeJson, row→entry mapping, summary builders (pure)
│   └── tests.rs         #   CRUD integration tests against an in-memory database
├── hotkeys.rs           # Hotkey translation and OS-filtered binding resolution
├── image_storage.rs     # ImageStorage — temp image file save/load for conversation history
├── mcp/                 # MCP client — rmcp-based tool server management
│   ├── mod.rs           #   Re-exports McpClient, McpError
│   └── client.rs        #   McpClient wrapper, McpError enum
├── menu_coordinator.rs  # MenuCoordinator — aggregates menu providers into ordered sections
├── notification.rs      # NotificationService — event-gated Tauri event emission
├── placeholder.rs       # PlaceholderService — template variable substitution and image injection
├── execution.rs         # PromptExecutionService — execution state machine (cancel, streaming, model resolution)
├── skill/               # SkillService — file-based skill loading + DB-backed versioning
│   ├── mod.rs           #   SkillService load/list/sync_versions; SkillError
│   └── parser.rs        #   YAML frontmatter + body splitter (pure)
├── speech/              # SpeechService — recording + transcription
│   ├── mod.rs           #   SpeechService state machine, SpeechError
│   ├── recorder.rs      #   cpal device discovery, sample-rate negotiation, WAV encoding
│   └── transcriber.rs   #   HTTP transcribe() for OpenAI/ElevenLabs + SttOptions
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

**Mutation methods**: `add_model`, `update_model` (upsert), `delete_model`, `add_prompt`, `update_prompt`, `delete_prompt`, `reorder_prompts`, `update_notifications`, `update_keymaps`, `update_menu_section_order`, `update_setting`. STT models live in the same `models` list (discriminated by `ModelConfig.model_type = Stt`) — use `add_model`/`update_model`/`delete_model`. `settings.speech_to_text_model: Option<String>` holds the id of the currently selected STT model; resolve via `ConfigService::resolve_stt_model()`.

**Prompt files**: All prompts live as `.md` files under `<config_dir>/prompts/{base,surfaces}/`. JSON config holds only paths. Six `PromptKind`s — `System`, `AboutYou`, `Environment`, `InputFormat`, `TitleGeneration`, `SpeechToText`. Read via `config.read_prompt(kind)` / write via `config.write_prompt(kind, content)`. `PromptStore` enforces path safety: relative-only, no `..`, must end with `.md`/`.markdown`, env-var refs (`${VAR}`) resolved at read time. Migrator extracts inline-string prompts from legacy `settings.json` into files on first load and renames legacy field names (`system_prompt` → `system`, `environment_section` → `environment`, `about_me` → `about_you`). The `prompts/base/about_me.md` file is renamed to `about_you.md` on disk; the historical `system.md` default `"You are a helpful assistant."` is replaced with the new anti-sycophancy default (user-edited content is not touched). Only `Environment` substitutes `{{date}}/{{time}}/{{timezone}}/{{os}}/{{active_app}}/{{recent_apps}}` placeholders — other prompts are sent verbatim.

**System prompt assembly**: `build_system_prompt_base` composes the runtime system message as XML-tagged sections joined with `\n\n` (no `---` separators). Order: raw system prompt → `<user_context>` (contains `Name: {preferred_name}` + about_you body) → `<environment>` → `<input_format>`. Empty/whitespace-only sections are dropped entirely; the `Name:` line is dropped if `preferred_name` is empty. `preferred_name` is a top-level `Settings` field (default empty, trimmed and clamped to 60 chars on `update_setting`).

### Testing pattern

Service tests use `tempfile::TempDir` to create isolated config directories. A `setup_test_dir()` helper copies the example settings into a temp dir. Tests that touch env vars should set and remove them within the test.

### ContextManagerService specifics

Manages an ordered list of `ContextItem` values (text and/or images) in memory. Session-only — no persistence to disk.

**No internal locking**: the struct is a plain `Vec<ContextItem>`. Thread safety is provided by the `Arc<Mutex<ContextManagerService>>` wrapper at the state-management layer (see `setup/state.rs`).

**No error enum**: all operations are infallible in-memory list manipulation.

**Constructor**: `ContextManagerService::new()` — returns an empty service.

**Text methods**: `set_context` (replace all), `append_context`, `get_context` (concatenates non-empty text with `\n`, returns `None` if no text), `has_context` (true if any Text items), `get_context_or_default`.

**Image methods**: `set_context_image` (replace all), `append_context_image`, `has_images`.

**General methods**: `clear`, `get_items` (cloned), `remove_item(index) -> bool`, `item_count`, `has_text_or_images`, `is_empty`.

**Key edge case**: `has_context()` checks for Text variant existence; `get_context()` additionally filters out empty-content items. Image-only context returns `None` from `get_context()`.

### SqliteHistoryService specifics

Persistent history storage via SQLite. Database file at `{app_data_dir}/history.db`. Survives app restarts.

**Schema**: two tables — `conversations` (metadata columns + `tree_json` blob for full conversation tree) and `conversation_images` (binary image data with foreign key to conversations, cascade delete). Schema migrations via `database.rs` with version table.

**No internal locking**: plain struct with `Database`. Thread safety provided by `Arc<Mutex<SqliteHistoryService>>` at the state-management layer.

**Constructor**: `SqliteHistoryService::new(database, max_entries)` — takes ownership of a `Database` instance.

**ID generation**: UUID v4 via `uuid::Uuid::new_v4()`.

**Timestamp format**: `chrono::Local::now().format("%Y-%m-%d %H:%M:%S")`.

**Max entries enforcement**: after each insert, deletes rows not in the top N by recency.

**Sorting**: `get_history()` returns entries ordered by `COALESCE(updated_at, created_at) DESC, rowid DESC`.

**Simple entries**: `add_entry()` inserts a single prompt execution (no tree_json).

**Conversation entries**: `add_conversation_entry()` serializes nodes/tree as JSON into `tree_json`, inserts images as BLOBs in a single transaction. Returns the entry ID. `update_conversation_entry()` replaces tree_json and re-inserts all images in a transaction.

**Image handling**: images received as base64 `ImagePayload`, decoded to BLOBs on save. On `get_entry_by_id()`, BLOBs are re-encoded to base64 and grouped into `node_images` (by node_id) and `context_images` (node_id = NULL) on `ConversationHistoryData`.

**Summary builders**: `build_input_summary` and `build_output_summary` derive summaries from nodes (last user/assistant content, truncated to 200 chars, with "+N more" suffix for multi-turn).

**Methods**: `new`, `add_entry`, `add_conversation_entry`, `update_conversation_entry`, `get_history`, `get_entry_by_id`, `get_last_item_by_type`, `get_last_quick_action`, `get_conversation_data`, `update_entry_title`, `clear`, `entry_count`.

**Error enum**: `HistoryError` with variants `EntryNotFound(String)` and `Database(rusqlite::Error)`.

### Hotkeys specifics

Pure functions (no struct/state) for translating keymap settings into `tauri-plugin-global-shortcut`-compatible shortcut strings.

**OS-aware translation**: `translate_shortcut(shortcut, os)` converts config format (`"cmd+f1"`) to plugin format (`"Command+F1"`). The `os` parameter (`"macos"`, `"linux"`, `"windows"`) affects modifier mapping — `cmd` becomes `Command` on macOS, `Super` on Linux/Windows.

**OS filtering**: `get_active_bindings(settings)` returns only bindings from `KeymapGroup` entries whose `context` matches the current OS (`std::env::consts::OS`). Returns `Vec<(translated_shortcut, action_name)>`.

**Validation**: shortcuts with fewer than 2 parts (no modifier) return `None` and are silently skipped.

**Key translation rules**: `cmd`→`Command`/`Super`, `ctrl`→`Control`, `shift`→`Shift`, `alt`→`Alt`, `meta`/`super`→`Super`. Key names: `f1`-`f20` uppercased, single letters uppercased, named keys mapped (`space`→`Space`, `esc`→`Escape`, `up`→`ArrowUp`, etc.).

### ImageStorage specifics

Manages temporary image files for conversation history. Saves base64-encoded image data to disk and loads it back, matching the original `image_storage.py` behavior.

**Temp directory**: `{app_data_dir}/temp_images/`. Created on `initialize()`, which clears any existing files first (called on app startup).

**Constructor**: `ImageStorage::new(app_data_dir)` — sets the temp directory path but does not create it. Call `initialize()` at startup.

**Save flow**: `save_image(base64_data, media_type)` decodes base64, writes to `img_{timestamp_ms}_{hash_12chars}.{ext}`. Returns the full file path as a `String`.

**Load flow**: `load_image(filepath)` reads the file, encodes to base64, infers media type from extension. Returns `(base64_data, media_type)`.

**Supported formats**: PNG, JPEG, GIF, WebP, BMP. Unknown types default to PNG.

**Cleanup**: `cleanup()` removes all files and recreates the empty directory. `initialize()` does the same on startup.

**Coordination with SqliteHistoryService**: `ImageStorage` handles transient image files during execution. Persistent image storage is handled directly by `SqliteHistoryService` via the `conversation_images` table (base64 → BLOB on save, BLOB → base64 on restore).

**Error variants**: `Io` (filesystem errors), `Base64Decode` (invalid base64 input).

**Methods**: `new`, `initialize`, `save_image`, `load_image`, `cleanup`.

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

**Streaming architecture**: The `complete_stream` trait method returns an owned `Stream`. The Tauri command layer (`commands/ai.rs`) bridges this to a `tauri::ipc::Channel<StreamEvent>`. The per-service `AiService` lock is released (via clone of the cheap `Arc`-backed handle) before stream iteration to avoid holding it for the duration of the HTTP response.

**SSE parser** (`sse.rs`): Converts a `reqwest::Response` byte stream into parsed `data:` line payloads. Handles line buffering, comment skipping, and `[DONE]` termination. Provider-agnostic — usable by any SSE-based provider.

**Error variants**: `ModelNotFound`, `ModelUnavailable`, `Authentication`, `Connection`, `RateLimit`, `ApiStatus { status, message }`, `Stream`, `Request`.

**OpenAiProvider** (`openai.rs`): Sends requests to `{base_url}/chat/completions`. Default base URL: `https://api.openai.com/v1`. Supports any OpenAI-compatible endpoint via the `base_url` config field. Model parameters (temperature, max_tokens, etc.) are merged into the request body when present.

### PromptExecutionService specifics

Orchestrates the full prompt execution pipeline: validate → resolve prompt → process placeholders → call AI → deliver result. Lives in `execution.rs`.

**Execution state tracking**: `is_executing: bool` and `current_execution_id: Option<String>`. Guards against concurrent execution — `start_execution()` returns `ExecutionError::AlreadyExecuting` if already busy.

**Constructor**: `PromptExecutionService::new()` — creates idle service.

**State methods**: `is_busy()`, `current_execution_id()`, `start_execution() -> Result<String, ExecutionError>` (generates UUID), `finish_execution()`.

**Resolution methods** (associated functions, take `&ConfigService`):
- `resolve_model(config, model_id)` — validates explicit model ID or falls back to `default_model`

**Error enum**: `ExecutionError` with variants `AlreadyExecuting`, `ModelNotFound(String)`, `ClipboardError(String)`, `AiError(String)`.

### Adding a new service

1. Create `services/<name>.rs` with error enum + struct + impl.
2. Add `pub mod <name>;` to `services/mod.rs`.
3. Wire into Tauri managed state in `lib.rs` (Task 4 pattern).
4. Add tests using `tempfile` for any file I/O.
