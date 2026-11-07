# Models

Data structures used across the app. See parent [`src-tauri/DOCS.md`](../../DOCS.md) for general conventions (serde derives, `rename_all`).

## Conventions

### Derive standard

Model structs derive `Debug, Clone` and `Serialize` / `Deserialize` as the direction they cross requires. Add `Default` when the struct has meaningful defaults. Add `PartialEq` only when needed for comparisons (e.g., enums used in assertions).

### Stable `settings.json`

Maps in `Settings` serialize sorted by key (`serialize_sorted`, `BTreeMap` for `ModelParameters.extra`), so `settings.json` is byte-stable across saves. Unknown top-level keys are kept in `Settings.extra`.

### Appearance and the icon-flag migration

`Settings.appearance` (`AppearanceSettings`, `#[serde(default)]`) holds `logo` (default `color`) and `icons` (default `mono`), both `Skin` (`color` | `mono`, serialized lowercase). `appearance.icons` replaced `notifications.monochromatic_notification_icons` and `surfaces.speech_to_text.monochromatic_widget_icon`: `parse_and_migrate` (`services/config/migrator.rs`) runs `migrate_monochromatic_flags` on every load, not only for legacy files. When either old key is present it sets `appearance.icons` to `mono` if one of them is `true`, else `color`, keeps `appearance.logo` and deletes both keys, so the next save writes the file without them.

`Settings.menu_digit_select_delay_ms` (`u32`, default 200, catalog range 100–1000, live) is how long the context menu waits for a second digit when a two-digit number is still possible.

`Settings.surfaces.settings_agent` (`SettingsAgentConfig`, `#[serde(default)]`) holds `generation: SettingsAgentGeneration` (`model_id`, `parameters`, `enabled_tools`: qualified `server.tool` names of the MCP tools the agent may call, `#[serde(default)]` empty, set only by the user through `update_surface_enabled_tools`; `quick_actions` keeps `QuickActionsGeneration`, which has no tools); an empty `model_id` falls back to the chat model at run time (`settings_agent/engine.rs`), `defaults.rs` does not fill it.

### Removed settings keys

`strip_removed_settings` (`services/config/migrator.rs`) runs in `parse_and_migrate` on every load, right after `migrate_monochromatic_flags` and before deserializing. It deletes the top-level keys `code_theme` and `debug_mode` (they would otherwise survive in `Settings.extra`) and every `keymaps` entry whose `context`, whitespace removed, is `os==windows`; it logs at `info` only when it removed something. Removed nested fields (`enabled_tools` of `quick_actions` and `title_generation`, `parameters` of `title_generation`) need no strip: `QuickActionsGeneration` and `TitleGeneration` do not declare them, so deserializing drops them and the next save writes the file without them.

### Environment variable list `env_vars`

`Settings.env_vars` (`Vec<EnvVar>`, `#[serde(default)]`, declared after `webview_providers`, so an older app keeps the key in `extra`) is the synced list of the variables settings reference as `${NAME}`. Values are never stored here (they live in `.env`). `EnvVar` fields: `name` (unique, passes `secrets::validate_name`), `description` (default empty), `secret` (default `false`, masks the value), `renamed_from` (default empty; each machine copies a missing value from the first previous name that has one). `validate` rejects an invalid or duplicate name with key `env_vars[<name>].name`; the catalog section is `environment`.

Seeding: `parse_and_migrate` sets `MigrationOutcome.seed_env_vars` when the raw JSON object lacks the key `env_vars`; `build_settings` then fills the list with one entry per name `missing_env::collect_missing(&settings, |_| false)` returns (sorted by name, empty `description` and `renamed_from`). `secret` is `true` when a use key ends with `.api_key` or contains `.env.` (MCP server env value). A referenced name that fails `validate_name` is skipped with a `warn` log, so it cannot make the load fail. An existing list, also `[]`, is never re-seeded.

### Serde defaults pattern

`surfaces.speech_to_text.recording_widget_at_cursor` (default `true` via `default_true` and the manual `Default` impl): the recording widget opens at the cursor and drags are not stored. `surfaces.speech_to_text.recording_widget_anchor` (`#[serde(default)]`, `PillAnchor`, default `above_right`) picks the side of the cursor: `above_right` puts the pill's bottom-left corner at the cursor; `above_*`/`below_*` × `right`/`center`/`left`.

`surfaces.speech_to_text.claude_target` (`#[serde(default)]`, `ClaudeTarget`: `webview` default | `desktop`) picks where the Claude pill target delivers; `desktop` labels the pill's Claude entry "Claude Desktop".

`surfaces.speech_to_text.stop_chat_target` (`#[serde(default)]`, `StopChatTarget`: `chat` default | `claude`) picks where the pill's Stop + chat button sends the transcript; `claude` delivers as the `webview:claude` target, so `claude_target` picks Claude Desktop or the Claude webview.

The settings JSON uses `snake_case` keys — no `rename_all` needed on settings structs (Rust fields already match).

For fields with non-trivial defaults, use named default functions:

```rust
#[serde(default = "default_true")]
pub show_tray_icon: bool,
```

For fields where the type's `Default` is correct (`false`, `0`, `None`, empty `Vec`), use bare `#[serde(default)]`.

### IPC model structs

Non-settings model files define types that cross the Rust↔TypeScript IPC boundary. Each has a hand-written TypeScript mirror in `src/lib/types/` (e.g. `context.rs` → `types/context.ts`, `message.rs` → `types/ai.ts`); event payloads emitted from services or commands (e.g. `HistoryChangedEvent`, mirrored in `src/lib/services/events.ts`) are mirrored next to their listener instead. Change both sides in the same change.

- Enums use `rename_all = "snake_case"` or `"lowercase"`; the attribute on the type decides, and the TS mirror must use the same strings.
- `ContextItem` is an internally tagged enum (`tag = "item_type"`), mirrored as a discriminated union, not an interface with optional fields; both members carry `number`, assigned per kind by `ContextManagerService`.
- `MessageContent` is `untagged`; `ContentPart` is tagged with `tag = "type"`.
- `Option<T>` maps to `T | null`, except where the Rust field is skipped when `None`: `ImageData.number` is `number?: number` on `ConversationImage` (`types/conversation.ts`). `serde_json::Value` maps to `unknown`.

### Adding new settings fields

1. Add the field to the appropriate struct with `#[serde(default)]` (or a named default fn).
2. Update the `Default` impl if the struct has one.
3. Add a catalog entry in `services/config/catalog.rs`, or list the path in `FILE_ONLY`; the coverage test (`uncovered_leaves`) fails otherwise.
4. Run `cargo test --lib` — the round-trip test will catch serialization issues.

Catalog paths are field names joined by `.`; a list item is `[]` (`models[].api_key`), an entry of the map `mcp_servers` is `*` (`mcp_servers.*.command`). Concrete paths used by errors and links: a list item of `models` or `webview_providers` is `[<id>]` (`models[abc].model`), any other list item `[<index>]`, a map entry its key (`mcp_servers.my-tools.command`); `catalog_path` turns a concrete path into its catalog path.

### Skill `offered_in`

Optional frontmatter field of `SKILL.md`, carried by `SkillFrontmatter`, `Skill`, `SkillSummary` and `SkillFull` as `Option<Vec<OfferedIn>>`. Values: `context_menu`, `pill`.

- Absent: offered in every place (`is_offered_in` returns `true`).
- `[]`: offered only in the chat.
- A list: offered only in the listed places.
- Unknown value (e.g. `chat`): the file is invalid (`SkillError::YamlParse`) and the skill is skipped on load.
- The chat `/skill` autocomplete ignores the field.

### Skill types

`Skill` is the loaded skill; it serializes without `file_path` (`#[serde(skip)]`) and with `skill_version_id` only when set (command `get_skill`, TS `Skill`). `SkillFull` is the settings-editor shape of the same skill: it carries `file_path` as a string and no `skill_version_id` (commands `list_skills_full`, `create_skill`, `update_skill`, `duplicate_skill`, `import_skill_file`; TS `SkillFull`). They stay two types because `SkillEditor` shows `file_path` and the JSON of both commands must keep its keys. `SkillSummary` is the list shape. `ResolveSkillResult` (`services/skill_message.rs`) is both the service result and the `resolve_skill_input` command return value.
