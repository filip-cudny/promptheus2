# Models

Data structures used across the app. See parent [`src-tauri/DOCS.md`](../../DOCS.md) for general conventions (serde derives, `rename_all`).

## Conventions

### Derive standard

Model structs derive `Debug, Clone` and `Serialize` / `Deserialize` as the direction they cross requires. Add `Default` when the struct has meaningful defaults. Add `PartialEq` only when needed for comparisons (e.g., enums used in assertions).

### Stable `settings.json`

Maps in `Settings` serialize sorted by key (`serialize_sorted`, `BTreeMap` for `ModelParameters.extra`), so `settings.json` is byte-stable across saves. Unknown top-level keys are kept in `Settings.extra`.

### Serde defaults pattern

`surfaces.speech_to_text.recording_widget_at_cursor` (default `true` via `default_true` and the manual `Default` impl): the recording widget opens at the cursor and drags are not stored. `surfaces.speech_to_text.recording_widget_anchor` (`#[serde(default)]`, `PillAnchor`, default `above_right`) picks the side of the cursor: `above_right` puts the pill's bottom-left corner at the cursor; `above_*`/`below_*` × `right`/`center`/`left`.

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
3. Run `cargo test --lib` — the round-trip test will catch serialization issues.

### Skill `offered_in`

Optional frontmatter field of `SKILL.md`, carried by `SkillFrontmatter`, `Skill`, `SkillSummary` and `SkillFull` as `Option<Vec<OfferedIn>>`. Values: `context_menu`, `pill`.

- Absent: offered in every place (`is_offered_in` returns `true`).
- `[]`: offered only in the chat.
- A list: offered only in the listed places.
- Unknown value (e.g. `chat`): the file is invalid (`SkillError::YamlParse`) and the skill is skipped on load.
- The chat `/skill` autocomplete ignores the field.

### Skill types

`Skill` is the loaded skill; it serializes without `file_path` (`#[serde(skip)]`) and with `skill_version_id` only when set (command `get_skill`, TS `Skill`). `SkillFull` is the settings-editor shape of the same skill: it carries `file_path` as a string and no `skill_version_id` (commands `list_skills_full`, `create_skill`, `update_skill`, `duplicate_skill`, `import_skill_file`; TS `SkillFull`). They stay two types because `SkillEditor` shows `file_path` and the JSON of both commands must keep its keys. `SkillSummary` is the list shape. `ResolveSkillResult` (`services/skill_message.rs`) is both the service result and the `resolve_skill_input` command return value.
