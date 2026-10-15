# Services

Business logic layer. Services own state and behavior; Tauri commands delegate to them.

## Layout

One file or directory per service; the file name is the service name. Directories where a file's place does not follow from its name:

- `ai/` — `provider.rs` (`AiProvider`, `CompletionRequest`, `StreamChunk`), `openai.rs` (Completions) and `openai_responses.rs` (Responses), `encoder.rs` (per-API capability encoding), `sse.rs` (SSE line parser)
- `execution/` — `lifecycle.rs` (`PromptExecutionService`, `ExecutionError`), `skill_executor.rs` and `conversation_executor.rs` (message preparation and result recording), `stream.rs` (stream and tool loop)
- `settings_sync/` — `git.rs` (`GitRunner`), `detect.rs`, `reload.rs`
- `speech/` — see [speech/DOCS.md](speech/DOCS.md); `mcp/` — see [mcp/DOCS.md](mcp/DOCS.md)

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

Services are plain structs (not singletons). They are created once at startup in `setup/state.rs` and placed into Tauri managed state. Services whose methods take `&mut self`, or that are replaced whole (`AiService` on settings reload), are behind `Arc<Mutex<…>>`; the others are managed without an outer `Mutex` (for example `ClipboardService`, `McpRegistry`, `RecordingWidget`, `SettingsSync`): they expose only `&self` methods and lock internally where they hold mutable state. Check `setup/state.rs` for the exact managed type before calling `state::<T>()`. The command layer is responsible for locking and calling service methods.

- **Constructor**: no single rule; `ConfigService`, `SkillService` and `UiStateService` use `Service::load(args)`, most others `new`, `Database` uses `open` and `McpRegistry` uses `start_all`.
- **Persistence**: no service exposes a public `save`. `ConfigService` writes inside `mutate`; `UiStateService` mutators write their file themselves.
- **Reload**: `ConfigService::reload` re-reads `settings.json`, replacing in-memory state.

### Window lookup

Tauri's `get_webview_window(label)` returns `None` when the window holds any webview whose label differs from the window label (`Window::is_webview_window`). `conversation-dialog` (toolbar and content children added with `add_child`) is such a window: look it up with `get_window` (window APIs) and `get_webview` (`eval`, navigation). `dialog::open_or_focus` returns `(tauri::Window, created)` for this reason.

### Settings sync

`settings_sync` runs the system `git` CLI with the login-shell environment (`shell_env`), never interactively: stdin null, `GIT_TERMINAL_PROMPT=0`, `ssh -o BatchMode=yes` unless the user configured ssh, per-command timeout with `kill_on_drop`. `detect()` finds the repo from the `settings.json`, `prompts`, `skills` symlinks of the config dir (none → `off`, different repos → `error`). A cycle: detect, `fetch`, `lock_local()` hook (which also runs `reload::reload_if_changed`: an external change of `settings.json`, such as a manual `git pull`/`reset` or a hand edit, is reloaded at the start of every cycle), `add -A` + `commit` on the target paths only (`*.tmp` excluded), `rebase --autostash @{u}`, `reload` hook for files changed by the rebase, unlock, `push` when ahead. A failed rebase is aborted and the engine pauses (`conflict`): `Periodic` triggers take `lock_local` (so an external change is still reloaded) and return without running git until a `Startup`/`Manual` cycle succeeds; the pause is in memory only. Locks, reload, events and toasts are supplied by the caller through `SyncHooks`. Logs and errors carry git stderr only, never file contents or diffs. The app hooks (`commands/settings_sync.rs`) lock `ConfigService` then `SkillService` (owned guards held across the local git steps), and lock history after them, in `lock_local` (for `reload_if_changed`) and in `reload`: lock order is config → skills → history; nothing may take config while holding skills. `reload::reload_changed` decides what to reload from the changed paths, compared canonically: the `settings.json` target → `ConfigService::reload` and a skills reload (`skills_order` lives in settings); a path under the `skills` target → skills reload and `prune_missing_skills`; `prompts` only → nothing (prompts are read from disk on use). Flags from `lock_local` and `reload` are OR-ed (`Reloaded::merge`). After the guards drop, shortcuts and autostart are re-applied and `settings-changed` / `skills-changed` (reason `sync`) are emitted; `AiService` is rebuilt by `apply_settings_reload`. `mcp_servers` and `show_tray_icon` changes apply only after restart. Cycles run on start, every 5 minutes (`setup/background.rs`) and on `sync_settings_now`; status goes out as `settings-sync-status-changed`, and an Error toast shows only when a cycle enters `conflict` or `error`.

### ClipboardService specifics

Reads text and images with the `arboard` crate; writes text through `write_text` (below). The struct holds `app: Option<AppHandle>` (`None` only in tests, via `without_app`): with an app, `set_text` goes through `write_text`; without, through `arboard`.

**Key pattern — no stored clipboard handle**: `arboard::Clipboard` is not `Send`/`Sync`, so it cannot be held across await points or stored in shared state. Each read creates a fresh `arboard::Clipboard` per call.

**Error variants**:
- `Unavailable` — clipboard is empty or content can't be read (soft error)
- `Access` — clipboard system is inaccessible (hard error)
- `ImageConversion` — image encode/decode failed

**Image pipeline**: `arboard::ImageData` (raw RGBA pixels) → `image::ImageBuffer` → PNG encode → base64 string. Returns `(base64_data, media_type)` tuple.

**Reads**: `get_text` trims and fails with `Unavailable` on empty; `get_text_raw` returns the text untouched (the raw-paste path); `has_image`, `has_file_list` and `get_image_base64` serve the paste-kind logic. `paste_kind(has_file_list, has_image)` decides what a paste into the context does: file list → nothing (files are not pasteable items, even when an image or text is also offered), else image, else text.

**Clipboard fingerprint**: `ClipboardService::fingerprint()` (blocking, call from `spawn_blocking`) returns a `ClipboardFingerprint`: non-empty text → `Text(hash)`; else raw image (`get_image` bytes plus size, never PNG-encoded) → `Image(hash)`; else non-empty file list → `Files(hash)`; else `Empty`. `ContentNotAvailable` or an empty value means "absent, try the next format"; any other arboard error (or no clipboard handle) → `Unreadable`, reading stops. Used by stop + chat: `store_pending_copy` keeps the snapshot with a uuid token; the command `copy_chat_answer_if_unchanged(token, text)` (`commands/clipboard.rs`) takes the slot, compares a fresh fingerprint and writes `text` only if the token is the current one and the fingerprints are equal and not `Unreadable`.

**Linux text writes**: `write_text` pipes the text to `wl-copy` when `WAYLAND_DISPLAY` is set, else to `xclip -selection clipboard`. When the tool is missing it logs `warn` (`clipboard subprocess failed`) and falls back to `tauri_plugin_clipboard_manager`; on GNOME Wayland that fallback writes the XWayland clipboard and the text does not reach Wayland apps, without an error. The `.deb` therefore depends on `wl-clipboard` (`bundle.linux.deb.depends` in `src-tauri/tauri.conf.json`).

### ConfigService specifics

**Load sequence** (`ConfigService::load`): `load_env` (`<config dir>/.env`, then `dotenvy::dotenv_override()` from the working directory and its parents, both overriding the process environment; sets `CONFIG_DIR`) → default assets copied when `settings.json` is missing → read and `parse_and_migrate` (schema migration in `migrator.rs`) → legacy prompt file moves and defaults → `migrate_model_params` → capability sanitizing → `ensure_surface_defaults` → `validate` → `write`. A model's `api_key` is a literal key or a `${VAR}` reference, resolved at use by `ModelConfig::resolved_api_key`; `write` serializes the settings as they are, with no key stripping.

**Mutations go through `ConfigService::mutate`**: it reloads `settings.json` when its bytes differ from the last read or written, runs the closure, then writes. No public `save` exists. A reload inside `mutate` is applied to runtime state by `commands::settings::apply_settings_reload`, called after the config guard is dropped. A failed reload fails the mutation and writes nothing. `settings.json` is written in place with `std::fs::write` because it may be a symlink into the settings repo; a temp-file rename would replace the symlink and stop sync.

**Mutators**: `update_*` and `add_model`/`update_model` (upsert)/`delete_model` change the in-memory settings only; they run inside `mutate`. STT models live in the same `models` list (discriminated by `ModelConfig.model_type = Stt`). The selected STT model is `surfaces.speech_to_text.model_id`; resolve it with `ConfigService::resolve_stt_model()`. Prompts are not settings entries: they are files (below).

**Prompt files**: All prompts live as `.md` files under `<config_dir>/prompts/{base,surfaces}/`. JSON config holds only paths. Six `PromptKind`s — `System`, `AboutYou`, `Environment`, `InputFormat`, `TitleGeneration`, `SpeechToText`. Read via `config.read_prompt(kind)` / write via `config.write_prompt(kind, content)`. `PromptStore` enforces path safety: relative-only, no `..`, must end with `.md`/`.markdown`, env-var refs resolved (`config/path.rs`) before those checks. Migrator extracts inline-string prompts from legacy `settings.json` into files on first load and renames legacy field names (`system_prompt` → `system`, `environment_section` → `environment`, `about_me` → `about_you`). The `prompts/base/about_me.md` file is renamed to `about_you.md` on disk; the historical `system.md` default `"You are a helpful assistant."` is replaced with the new anti-sycophancy default (user-edited content is not touched). Only `Environment` substitutes `{{date}}`/`{{time}}`/`{{timezone}}`/`{{os}}`/`{{active_app}}`/`{{recent_apps}}` placeholders (`placeholder_registry.rs`) — other prompts are sent verbatim.

**System prompt assembly**: `build_system_prompt_base` composes the runtime system message as XML-tagged sections joined with `\n\n` (no `---` separators). Order: raw system prompt → `<user_context>` (contains `Name: {preferred_name}` + about_you body) → `<environment>` → `<input_format>`. Empty/whitespace-only sections are dropped entirely; the `Name:` line is dropped if `preferred_name` is empty. `preferred_name` is a top-level `Settings` field (default empty, trimmed and clamped to 60 chars on `update_setting`).

### Testing pattern

Service tests use `tempfile::TempDir` to create isolated config directories. A `setup_test_dir()` helper copies the example settings into a temp dir. Tests that touch env vars should set and remove them within the test.

### ContextManagerService specifics

Manages an ordered list of `ContextItem` values (text and/or images) in memory. Session-only — no persistence to disk.

**No internal locking**: the struct is a plain `Vec<ContextItem>`. Thread safety is provided by the `Arc<Mutex<ContextManagerService>>` wrapper at the state-management layer (see `setup/state.rs`).

**No error enum**: all operations are infallible in-memory list manipulation.

**Constructor**: `ContextManagerService::new()` — returns an empty service.

**Labelled context**: `labelled_context()` returns `None` without text items, otherwise one `<pasted-text name="Text #{number}">\n{content}\n</pasted-text>` block per text item (empty content included, images skipped) joined with `\n\n`; `pasted_text_block` builds a block and is shared with chat message attachments. Consumers: skill prompts (`prepare_skill_messages` wraps it as `{system_prompt}\n\n<context>\n{blocks}\n</context>` in the system message and labels context images `[Context Image #{number}]` in the user message) and command `get_labelled_context`. `get_context()` stays the raw `\n` join without tags and serves "Copy context text" (`get_context_text`) and `{{context}}`.

**Numbers**: each item gets a per-kind `number` (`Text #n`, `Image #n`) from two counters (`next_text_number`, `next_image_number`): assigned on add, kept on `remove_item`, reset to 1 by `clear` and by every replace (`set_context`, `set_context_image`, so also `set_context_from_clipboard` and the `set_context_value` hotkey). Pill, context menu and context editor label items from it. Context hotkeys emit `context-changed` through `emit_context_changed` (version bump, pill refresh).

**`has_context` semantics**: `has_context()` is `!items.is_empty()`, so it is true for an image-only context; `get_context()` concatenates the non-empty text items with `\n` and returns `None` when there are none, so an image-only context returns `None` from it. Use `has_images()` / `get_context()` when the kind matters.

**Context version and pill**: `commands/context.rs` owns a static `CONTEXT_VERSION`; `current_context_version()` reads it and `emit_context_changed` is the only place that bumps it. After emitting `context-changed` it spawns `RecordingWidget::refresh_model`, which resizes and re-pushes the pill model when the pill is shown. Pill item removal and Paste go through `remove_item_for_pill` / `paste_item_for_pill` there; a removal whose version differs from the current one removes nothing.

### SqliteHistoryService specifics

Persistent history storage via SQLite. Database file at `{app_data_dir}/history.db`. Survives app restarts.

**Schema**: two tables — `conversations` (metadata columns + `tree_json` blob for full conversation tree) and `conversation_images` (binary image data with foreign key to conversations, cascade delete). Schema migrations via `database.rs` with version table.

**No internal locking**: plain struct with `Database`. Thread safety provided by `Arc<Mutex<SqliteHistoryService>>` at the state-management layer.

**Constructor**: `SqliteHistoryService::new(database, retention_days)` — takes ownership of a `Database` instance. `retention_days` comes from `Settings::history_retention_days`.

**ID generation**: UUID v4 via `uuid::Uuid::new_v4()`.

**Timestamp format**: `chrono::Local::now().format("%Y-%m-%d %H:%M:%S")`.

**Retention**: time-based, not count-based — there is no cap on the number of entries. `enforce_retention()` deletes rows whose `COALESCE(updated_at, created_at)` is older than `datetime('now', 'localtime', '-N days')`; `retention_days == 0` disables it entirely (the default). `'localtime'` is mandatory because timestamps are written as `chrono::Local`; comparing against UTC `now` would prune fresh entries at positive offsets. The sweep is **not** on the insert path — it runs once at startup (`setup/state.rs`) and again from the `update_history_retention` command when the user changes the setting. Deletes cascade to `conversation_images` and are mirrored into `conversations_fts` by the `conversations_fts_ad` trigger.

**Sorting**: `get_history()` returns entries ordered by `COALESCE(updated_at, created_at) DESC, rowid DESC`.

**Simple entries**: `add_entry()` inserts a single prompt execution (no tree_json).

**Conversation entries**: `add_conversation_entry()` serializes nodes/tree as JSON into `tree_json`, inserts images as BLOBs in a single transaction. Returns the entry ID. `update_conversation_entry()` replaces tree_json and re-inserts all images in a transaction.

**Source columns**: `conversations` has `invocation_source` (`context_menu` = skill click, `alternative_execute` = Shift+click or the action menu's "Run in dialog" (with `output_route` `chat`) or "Run with transcription" (voice run, `clipboard`); the pair tells them apart, `chat` = other conversation-dialog rows, `pill` = skill run started from the recording pill) and `output_route` (`clipboard` = skill executor rows, `chat` = conversation-dialog rows, `clipboard_chat` = pill skill run with stop + chat, written only before the chat opened at once on stop + chat; stored rows keep it). Only `add_conversation_entry()` writes them (the executor passes `clipboard`, the command passes `chat`); `update_conversation_entry()` leaves them unchanged. Both are `NULL` in rows from before schema v7 and in speech rows (`add_entry`). Plain `TEXT` without `CHECK`, so new values need no schema migration. `run_skill_stream` takes `output_route` and `notify_success` (`false` skips the success toast and also shows the prepare-error toast, for the pill) and returns the created history entry id (`None` for a cancelled run); `execute_skill` passes `clipboard` and `true`. No reader yet (statistics via SQL).

**Image handling**: images received as base64 `ImagePayload`, decoded to BLOBs on save. On `get_entry_by_id()`, BLOBs are re-encoded to base64 and grouped into `node_images` (by node_id, no number) and `context_images` (node_id = NULL) on `ConversationHistoryData`. Context rows are saved with `image_index = number - 1` and read back with `ImageData.number = image_index + 1`. `build_messages_from_tree` labels context images `[Context Image #{number}]`; an image without a number gets `max(numbers) + k` for the k-th unnumbered one.

**Summary builders**: `build_input_summary` and `build_output_summary` derive summaries from nodes (last user/assistant content, truncated to 200 chars, with "+N more" suffix for multi-turn).

**Previews are not content**: for conversation entries the `input_content` / `output_content` columns hold only those summaries, and they surface on `HistoryEntry` as `input_preview` / `output_preview` to keep that visible at every call site. The full text lives in `tree_json` and is reachable through `resolve_full_content(entry_id, part)` (one row read, no image decoding) or through `conversation_data` on `get_entry_by_id`. Anything that copies, exports or re-executes text must go through `resolve_full_content`; the previews are for rendering only. Simple entries (`add_entry`, i.e. speech) store the whole value in those columns, which is why `resolve_full_content` falls back to them when there is no tree. Search (`history_search.rs`) matches previews, so a response is only searchable in its first 200 characters.

**Compaction**: `DELETE` never shrinks the SQLite file — freed pages land on the freelist and the database is in WAL mode, so a bulk delete also inflates `history.db-wal`. `compact_after_bulk_delete()` runs after any bulk delete (retention change, `clear_history`): first `PRAGMA wal_checkpoint(TRUNCATE)` (cheap, no exclusive lock), then `VACUUM` only if `reclaimable > 100 MB` **and** `freelist_count / page_count > 0.25` — the conjunction avoids vacuuming a tiny database on ratio alone, or a multi-GB one where 100 MB is noise. `vacuum()` is also exposed unconditionally through the `compact_history_database` command for the manual "Compact database" button. `VACUUM` needs ~2× the database size on disk and holds an exclusive lock, so it never runs on the startup sweep or on the insert path. Both commands wrap the blocking work in `tokio::task::block_in_place` (lock is acquired with `.await` first — nesting `block_on` inside a runtime thread would panic). `storage_stats()` reports `page_count * page_size` and `freelist_count * page_size` — all three pragmas are database-header reads (O(1)), so the settings UI can refresh them on every `history-changed` event for free. Deliberately excludes an entry count: `SELECT COUNT(*)` is a full scan and `history-changed` fires on every prompt execution.

**Full-text search**: `conversations_fts` (FTS5 over `title`, `skill_name`, `input_content`, `output_content`) is kept in sync by the `conversations_fts_ai`/`_ad`/`_au` triggers; `search_fts` joins it to `conversations` on `rowid` and orders by `bm25` (weights 10, 5, 2, 1). The query string reaches `MATCH` as given, so callers (`history_search.rs`) must build a valid FTS5 expression. `conn()` exposes the connection to commands that pass it to `AudioClipStore`, `SkillService::ensure_version` / `prune_missing_skills` and the settings-sync reload.

**Error enum**: `HistoryError` with variants `EntryNotFound(String)` and `Database(rusqlite::Error)`. `update_entry_result` and `delete_entry` return `EntryNotFound` for an unknown id.

### Hotkeys specifics

Mostly free functions translating keymap settings into `tauri-plugin-global-shortcut`-compatible shortcut strings; the one state is `ShortcutActionMap` (shortcut → action, behind an `RwLock`), written by `reload_shortcuts` and read by the global-shortcut handler in `setup/shortcuts.rs`, which then calls `execute_hotkey_action`.

**OS-aware translation**: `translate_shortcut(shortcut, os)` converts config format (`"cmd+f1"`) to plugin format (`"Command+F1"`). The `os` parameter (`"macos"` or anything else, in practice `"linux"`) affects only `cmd`: `Command` on macOS, `Super` otherwise.

**OS filtering**: `get_active_bindings(settings)` returns only bindings from `KeymapGroup` entries whose `context` matches the current OS (`std::env::consts::OS`). Returns `Vec<(translated_shortcut, action_name)>`.

**Validation**: shortcuts with fewer than 2 parts (no modifier) return `None` and are silently skipped.

**Key translation rules**: `cmd`→`Command`/`Super`, `ctrl`→`Control`, `shift`→`Shift`, `alt`→`Alt`, `meta`/`super`→`Super`. Key names: `f<N>` uppercased, single letters uppercased, named keys mapped (`space`→`Space`, `esc`→`Escape`, `up`→`ArrowUp`, etc.).

**Backends**: on GNOME Wayland (`gnome_shell::is_gnome_wayland()`) shortcuts are grabbed by the GNOME Shell extension; elsewhere by `tauri-plugin-global-shortcut`. `to_gtk_accelerator` converts the `get_active_bindings` output (`Control+F1`, `ArrowUp`) to GTK accelerators (`<Control>F1`, `Up`) for the extension; an unknown key returns `None`. `setup/shortcuts.rs` selects the backend. See [linux-wayland-gnome-extension.md](../../../docs/gotchas/linux-wayland-gnome-extension.md).

### GnomeShell specifics

Linux-only D-Bus client for the `com.promptheus.Shell` interface served by the GNOME Shell extension (`linux/gnome-shell-extension/`). Uses `zbus`.

- `Shell` (`#[zbus::proxy]`, interface `com.promptheus.Shell`): `SetShortcuts`, `GetPointer`, `PlaceWindow`, `PlaceWindowAnchored`, `PlaceWindowOnParent`, `GetFocusedWmClass`, `ShowToast(level, title, message, monochromatic)`, `ShowRecordingWidget(x, y, has_position, monochromatic) → (x, y, work_x, work_y, work_width, work_height)` (placed origin and work area of the monitor it landed on), `UpdateRecordingWidget(state, level, elapsed_ms)`, `HideRecordingWidget`; signals `ShortcutActivated`, `Ready`, `RecordingWidgetAction` and `RecordingWidgetMoved(x, y, work_x, work_y, work_width, work_height)`.
- The session connection lives in a process-wide `OnceLock`, so callers without state (`frontmost_app::detect()`) reach it through `proxy()` (async) or `blocking_proxy()`.
- Recording pill additions: method `UpdateRecordingWidgetModel(a(ss), s, s, b, a(ussss), u, t, i, i)` (`update_recording_widget_model`: targets, selected id and label, target lock, items `(index, kind, label, preview, image_path)`, item rows, context version, x, y; no dropdown direction: the model carries none, see [speech/DOCS.md](speech/DOCS.md)) and signals `RecordingWidgetTargetSelected(s)` / `RecordingWidgetItemRemoved(u, t)` consumed in `setup/shortcuts.rs`; `RecordingWidgetAction` also carries `paste` and `stop_chat`. A failed call is logged once per recording by `push_model`. See [speech/DOCS.md](speech/DOCS.md).
- `is_gnome_wayland()` is the backend selector: `XDG_SESSION_TYPE=wayland` and `XDG_CURRENT_DESKTOP` contains `GNOME`.
- `place_window(title, x, y, activate, show)` (context menu) and `place_window_anchored(title, right, bottom, activate, show)` (notification window) are the only callers of `PlaceWindow` and `PlaceWindowAnchored`; `place_window_on_parent(title, dx, dy, activate, show)` (provider menu, palette, backdrop) places a window relative to its transient parent, which `set_transient_parent(child, parent)` sets on the GTK main thread; all go through `call_placement`. With `show = Some(win)` the call is sent before `win.show()` and the reply awaited after it; with `None` it is a plain call to an already visible window.
- `frontmost_app` on Wayland reads `GetFocusedWmClass`; it returns an empty string when the extension is unreachable.
- Shortcuts run in `setup/shortcuts.rs` `run_shell_shortcuts`: `SetShortcuts` after every `Ready`; it falls back to the settings passed to `register` before `ConfigService` is managed.

### ImageStorage specifics

Manages temporary image files for conversation history. Saves base64-encoded image data to disk and loads it back.

**Temp directory**: `{app_data_dir}/temp_images/`. Created on `initialize()`, which clears any existing files first (called on app startup).

**Constructor**: `ImageStorage::new(app_data_dir)` — sets the temp directory path but does not create it. Call `initialize()` at startup.

**Save flow**: `save_image(base64_data, media_type)` decodes base64, writes to `img_{timestamp_ms}_{hash_12chars}.{ext}`. Returns the full file path as a `String`.

**Load flow**: `load_image(filepath)` reads the file, encodes to base64, infers media type from extension. Returns `(base64_data, media_type)`.

**Supported formats**: PNG, JPEG, GIF, WebP, BMP. Unknown types default to PNG.

**Cleanup**: `cleanup()` removes all files and recreates the empty directory. `initialize()` does the same on startup.

**Coordination with SqliteHistoryService**: `ImageStorage` handles transient image files during execution. Persistent image storage is handled directly by `SqliteHistoryService` via the `conversation_images` table (base64 → BLOB on save, BLOB → base64 on restore).

**Error variants**: `Io` (filesystem errors), `Base64Decode` (invalid base64 input).

### NotificationService specifics

Holds an `AppHandle`.

**Event gating**: `notify()` checks `NotificationSettings.events` before showing. If the event is disabled in settings, the notification is silently dropped. Error-level notifications bypass the gate.

**Display**: `notify()` calls `commands::notification::show_notification`. On GNOME Wayland it routes to the extension's `ShowToast` (falling back to the notification webview window if the call fails); elsewhere it uses the notification webview window. See `docs/gotchas/linux-wayland-gnome-extension.md`.

**`is_event_enabled` mapping**: maps the string event names (13) (e.g., `"prompt_execution_success"`, `"clipboard_copy"`) to the corresponding bool field on `NotificationEvents`. Unknown event names return `true` (safe default — show rather than hide).

### MenuCoordinator specifics

Aggregates `MenuItemProvider` trait implementors (defined in `traits.rs`) into a flat, ordered list of `MenuItem`s. Section order comes from `ConfigService.settings().menu_section_order`.

**Built-in virtual sections**: `"chat"`, `"skills"` (alias `"prompts"`), `"models"` and `"settings"` are handled directly (not via providers). Skills come from the registered providers with the dynamic ones (`ContextMenuProvider`, `LastInteractionMenuProvider`, `SpeechMenuProvider`) excluded; `models` and `settings` are built from `ConfigService`. The ids `speech`, `lastInteraction` and `context` map to the dynamic providers' names.

**Provider sections**: Any other section ID is mapped to a provider name (`resolve_provider_name`); the items come from the first registered provider whose `provider_name()` matches and that has items. Providers are added at startup via `add_provider()`.

**Separator placement**: A `separator_after = true` flag is set on the last item of each non-final section. No trailing separator.

`update_context_items` pushes context items to the `ContextMenuProvider`; `providers_mut` gives access to the registered providers.

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

**Testing**: Tests use mock processors (registered via the trait) to avoid system clipboard dependency. `ContextManagerService` is used directly since it's in-memory. `ClipboardService` is passed but unused by mock processors.

### AiService specifics

Multi-provider LLM service. Lives in `ai/` subdirectory with one file per provider. Uses `reqwest` for HTTP (not provider-specific SDKs) so a single dependency serves all providers.

**Provider trait** (`provider.rs`): `AiProvider` defines `complete(CompletionRequest) -> Result<String>` and `complete_stream(CompletionRequest) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>>>>>`. Providers are framework-independent — no Tauri imports.

**Provider registry**: `AiService::new(models)` iterates `ModelConfig` entries and matches on the `Provider` enum. Only `Provider::Openai` builds a client; `Anthropic`, `Gemini` and `ElevenLabs` models are recorded in `unavailable_models` with a reason ("provider not yet supported", "speech-to-text only"), as are OpenAI models whose provider fails to build (missing API key). The app still starts.

**API mode**: the model's `ApiMode` picks the provider and the capability encoder: `Responses` → `OpenAiResponsesProvider` (`{base_url}/responses`) with `OpenAiResponsesEncoder`, `Completions` → `OpenAiProvider` (`{base_url}/chat/completions`) with `OpenAiCompletionsEncoder`.

**Adding a new provider**:
1. Create `services/ai/<provider>.rs` implementing `AiProvider`.
2. Replace the `unavailable_models` arm of `AiService::new()` for that `Provider` variant (the variant already exists in `models/settings.rs`).

**Streaming architecture**: The `complete_stream` trait method returns an owned `Stream`. The Tauri command layer (`commands/ai.rs`) bridges this to a `tauri::ipc::Channel<StreamEvent>`. The per-service `AiService` lock is released (via clone of the cheap `Arc`-backed handle) before stream iteration to avoid holding it for the duration of the HTTP response.

**SSE parser** (`sse.rs`): Converts a `reqwest::Response` byte stream into parsed `data:` line payloads. Handles line buffering, comment skipping, and `[DONE]` termination. Provider-agnostic — usable by any SSE-based provider.

**Error variants**: `ModelNotFound`, `ModelUnavailable`, `Authentication`, `Connection`, `RateLimit`, `ApiStatus { status, message }`, `Stream`, `Request`.

**OpenAiProvider** (`openai.rs`) and **OpenAiResponsesProvider** (`openai_responses.rs`): Default base URL: `https://api.openai.com/v1`. Supports any OpenAI-compatible endpoint via the `base_url` config field. Model parameters (temperature, max_tokens, etc.) are merged into the request body when present.

### PromptExecutionService specifics

Holds the state of the running skill execution and the live (streaming chat) execution; the message preparation lives in `skill_executor.rs` and `conversation_executor.rs`, the stream and tool loop in `stream.rs`. Lives in `execution/`.

**Execution state**: one skill execution at a time. Starting while one runs returns `ExecutionError::AlreadyExecuting`; starting creates an execution id (UUID) and a `watch` cancel channel; finishing clears both and the live execution. `is_busy()` and `executing_skill_id()` report it, `cancel_execution()` signals the cancel channel. The live execution (`LiveExecution`, `start_live`, `cancel_live`, `clear_live`) has its own cancel channel and an `ExecutionSnapshot` of the streamed text.

**Model resolution** (associated functions, take `&ConfigService`): an explicit model id must exist in `models`, else `ModelNotFound`; without one, `resolve_model` falls back to `surfaces.chat.generation.model_id`, `resolve_quick_action_model` to `surfaces.quick_actions.generation.model_id` then the chat model, `resolve_title_generation_model` to `surfaces.title_generation.generation.model_id` then the chat model.

**Error enum**: `ExecutionError` with variants `AlreadyExecuting` and `ModelNotFound(String)`.

### Adding a new service

1. Create `services/<name>.rs` with error enum + struct + impl.
2. Add `pub mod <name>;` to `services/mod.rs`.
3. Wire into Tauri managed state in `setup/state.rs`.
4. Add tests using `tempfile` for any file I/O.
