# Models

Data structures used across the app. See parent [`src-tauri/DOCS.md`](../../DOCS.md) for general conventions (serde derives, `rename_all`).

## Directory Structure

```
models/
├── mod.rs          # Module declarations
└── settings.rs     # Full settings.json schema (Settings + sub-structs)
```

## Conventions

### Derive standard

All model structs derive `Debug, Clone, Serialize, Deserialize`. Add `Default` when the struct has meaningful defaults. Add `PartialEq` only when needed for comparisons (e.g., enums used in assertions).

### Serde defaults pattern

The settings JSON uses `snake_case` keys — no `rename_all` needed on settings structs (Rust fields already match).

For fields with non-trivial defaults, use named default functions:

```rust
#[serde(default = "default_true")]
pub show_tray_icon: bool,
```

For fields where the type's `Default` is correct (`false`, `0`, `None`, empty `Vec`), use bare `#[serde(default)]`.

### Settings struct hierarchy

`Settings` is the root. Sub-structs are not nested modules — all live in `settings.rs`:

- `Settings` — top-level, one per app
- `ModelConfig`, `ApiKeySource`, `ModelParameters` — AI model configuration
- `SpeechToTextModel` — STT model
- `PromptData`, `PromptMessage` — prompt definitions
- `KeymapGroup` — OS-specific hotkey bindings (`HashMap<String, String>`)
- `NotificationSettings`, `NotificationEvents`, `NotificationColors` — notification config
- `DescriptionGenerator` — auto-description model + prompt

### Adding new settings fields

1. Add the field to the appropriate struct with `#[serde(default)]` (or a named default fn).
2. Update the `Default` impl if the struct has one.
3. Run `cargo test --lib` — the round-trip test will catch serialization issues.
