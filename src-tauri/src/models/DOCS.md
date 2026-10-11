# Models

Data structures used across the app. See parent [`src-tauri/DOCS.md`](../../DOCS.md) for general conventions (serde derives, `rename_all`).

## Directory Structure

```
models/
├── mod.rs          # Module declarations
├── settings.rs     # Full settings.json schema (Settings + sub-structs)
├── menu.rs         # MenuItemType enum, MenuItem struct
├── execution.rs    # ErrorCode enum, ExecutionResult struct
├── context.rs      # ContextItem tagged enum (Text / Image variants)
├── skill.rs        # SkillFrontmatter, Skill, SkillSummary, SkillFull, OfferedIn
├── message.rs      # ProcessedMessage, MessageContent, ContentPart — LLM message format
└── history.rs      # HistoryEntryType, InvocationSource, OutputRoute, HistoryEntry,
                    #   ConversationHistoryData, SerializedConversationNode, ImagePayload
```

## Conventions

### Derive standard

All model structs derive `Debug, Clone, Serialize, Deserialize`. Add `Default` when the struct has meaningful defaults. Add `PartialEq` only when needed for comparisons (e.g., enums used in assertions).

### Stable `settings.json`

Maps in `Settings` serialize sorted by key (`serialize_sorted`, `BTreeMap` for `ModelParameters.extra`), so `settings.json` is byte-stable across saves. Unknown top-level keys are kept in `Settings.extra`.

### Serde defaults pattern

`surfaces.speech_to_text.recording_widget_at_cursor` (default `true` via `default_true` and the manual `Default` impl): the recording widget opens with its top-left corner at the cursor and drags are not stored.

The settings JSON uses `snake_case` keys — no `rename_all` needed on settings structs (Rust fields already match).

For fields with non-trivial defaults, use named default functions:

```rust
#[serde(default = "default_true")]
pub show_tray_icon: bool,
```

For fields where the type's `Default` is correct (`false`, `0`, `None`, empty `Vec`), use bare `#[serde(default)]`.

### IPC model structs

Non-settings model files define types that cross the Rust↔TypeScript IPC boundary. Each Rust file has a TypeScript mirror in `src/lib/types/`:

| Rust file | TypeScript file | Serde strategy |
|-----------|----------------|----------------|
| `menu.rs` | `types/menu.ts` | `rename_all = "snake_case"` on `MenuItemType` |
| `execution.rs` | `types/execution.ts` | `rename_all = "snake_case"` on `ErrorCode` |
| `context.rs` | `types/context.ts` | `tag = "item_type"` + `rename_all = "lowercase"` — internally tagged enum |
| `message.rs` | `types/ai.ts` | `untagged` on `MessageContent`, `tag = "type"` on `ContentPart` |
| `history.rs` | `types/history.ts` | `rename_all = "lowercase"` on `HistoryEntryType`; `rename_all = "snake_case"` on `InvocationSource` (`types/history.ts`, values `context_menu`, `alternative_execute`, `chat`, `pill`); `OutputRoute` values `clipboard`, `chat`, `clipboard_chat`; `OutputRoute` has no TS counterpart |

TypeScript mirrors use `T | null` for `Option<T>` and `unknown` for `serde_json::Value`. `ContextItem` is a discriminated union (not an interface with optional fields).

### Settings struct hierarchy

`Settings` is the root. Sub-structs are not nested modules — all live in `settings.rs`:

- `Settings` — top-level, one per app
- `ModelConfig`, `ApiKeySource`, `Provider`, `ModelParameters` — AI model configuration
- `SpeechToTextModel` — STT model
- `KeymapGroup` — OS-specific hotkey bindings (`HashMap<String, String>`)
- `NotificationSettings`, `NotificationEvents`, `NotificationColors` — notification config
- `DescriptionGenerator` — auto-description model + prompt

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
