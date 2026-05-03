# Settings UI

Settings dialog window — sidebar navigation + per-section content. Lives in its own Tauri window (`settings-dialog` label, entry `settings-dialog.html` → `src/windows/settings-dialog/App.svelte`).

Design principles are codified in [`migration/settings-ux.md`](../../../../../migration/settings-ux.md). This doc captures the patterns actually applied here so new sections stay consistent.

## Directory Structure

```
settings/
├── SettingsSidebar.svelte    # Left nav, exports SettingsSection union + SIDEBAR_ITEMS
├── SectionModels.svelte      # Models section: list pane + editor pane
├── ModelList.svelte          # Grouped model list, "Add" split-button (text/STT)
├── ModelEditor.svelte        # Single-model form: basic / connection / capabilities / parameters / danger
├── ParametersKnown.svelte    # Sliders + toggles for OpenAI-style known params
├── ParametersCustom.svelte   # Free-form key/type/value rows for arbitrary params
└── EnvRefChip.svelte         # ${ENV_VAR} reference indicator (resolved/missing)
```

Surrounding pieces (not in this dir):

- `src/windows/settings-dialog/App.svelte` — root: mounts sidebar + active section, calls `store.init()` / `store.destroy()`.
- `src/lib/stores/settings.svelte.ts` — single shared rune store (`getSettingsStore()`); subscribes to backend `settings-changed` events.
- `src/lib/services/settings.ts` — typed `invoke()` wrappers for `get_settings`, `update_model`, etc.
- `src/lib/services/settingsDialog.ts` — `openSettingsWindow(section?)` + `checkEnvVar(name)`.

## Conventions

### Adding a new section

1. Add the id to the `SettingsSection` union and `SIDEBAR_ITEMS` array in `SettingsSidebar.svelte`. Set `enabled: true` only when the section actually renders.
2. Create `Section<Name>.svelte` here and branch on it in `src/windows/settings-dialog/App.svelte`'s `{#if activeSection === ...}`.
3. Read state from `getSettingsStore()` — never `invoke("get_settings")` directly from a component.
4. Write through `$lib/services/settings.ts` helpers; the store auto-refreshes on the backend's `settings-changed` event.

Disabled sidebar items render with `tooltip: "Coming soon"` and are not clickable. Order in `SIDEBAR_ITEMS` is the user's mental model (General → Models → … → Advanced last), not alphabetical.

### Persistence — auto-save with debounce

No Save/Cancel buttons. Every input persists on change:

- **Text inputs** (display name, model id, base URL, API key, group, custom param values) — debounced via `scheduleSave(false)`. Debounce is `store.settings.number_input_debounce_ms` (default 200ms).
- **Discrete controls** (segmented buttons, selects, checkboxes, slider toggles) — `scheduleSave(true)` = persist immediately, no debounce.
- **Range sliders** — fire on every `oninput`; rely on the user releasing the slider for the final value. The known-parameter override checkbox is the gate: unchecking it sends `null` to drop the override entirely.

The pattern in `ModelEditor.svelte`:

```ts
let saveTimer: ReturnType<typeof setTimeout> | null = null;

function scheduleSave(immediate: boolean) {
  if (saveTimer) clearTimeout(saveTimer);
  if (immediate) persist();
  else saveTimer = setTimeout(persist, debounceMs);
}
```

`onDestroy` must clear the timer to avoid a stale write after the component unmounts.

### Draft state, not direct mutation

`ModelEditor` clones the incoming `model` prop into a local `$state` `draft` and edits the draft. A `$effect(() => { const m = model; untrack(() => { draft = structuredClone(m); ... }) })` resets the draft when the parent swaps to a different model. The parent uses `{#key selectedModel.id}` so switching models fully remounts the editor, which combined with `untrack` prevents stale-draft leaks across selections.

`structuredClone` (not spread) — required because `parameters` is nested.

### Validation

Inline only. `ModelEditor.validate()` populates `validationErrors: Record<string, string>` and inputs get `class:error={validationErrors.<field>}`. `persist()` early-returns on invalid state — the user sees the red border and field error, no toast, no modal.

Custom param errors live in `customErrors` keyed by `entry.id` (the parse function in `ParametersCustom.svelte`'s module script returns `{ extra, errors }`).

### Sections as cards

Inside a section, group related fields in `<section class="card">` with an uppercase `<h3>` heading (e.g., "Basic", "Connection", "Capabilities", "Parameters", "Danger zone"). Card styles live in `ModelEditor.svelte`'s scoped style — copy them when adding a new section to keep visual rhythm consistent.

### Danger zone

Last card, red border (`border-color: rgba(217, 115, 115, 0.3)`), separated visually. Destructive actions use a two-step inline confirm — never an OS dialog or a modal-on-modal.

```
[Delete model]  →  "Delete X? [Cancel] [Yes, delete]"
```

If the action affects other state (e.g., a model referenced by a surface), surface that warning inline in the confirm prompt — don't block, just inform.

### Labels & controls

- Left-align labels, full-width controls below (single-column form). For very short selects/segmented controls (Type), `width: fit-content`.
- Label = what it does ("Store request/response on provider"). `<p class="helper">` underneath = one-line tradeoff/effect.
- Toggle labels describe the **state**, not the action. Checkbox sits left of the text, both clickable via the wrapping `<label>`.
- `lucide-svelte` icons only — never raw SVG. Sizes from `$lib/constants/ui.ts` (`ICON_SIZE.sm`, `.md`).

### "Override or use default" pattern

`ParametersKnown.svelte` shows each param with a leading checkbox. Unchecked = inherit provider default (sends `null`); checked = enable the slider/input pre-filled with a sensible default (e.g., `temperature: 0.7`). This keeps the form short and makes "I haven't touched this" visually distinct from "I set it to 0".

### Env var references

API key fields accept literal secrets *or* `${VAR_NAME}` references. `EnvRefChip` parses the value, calls `check_env_var` on the backend, and shows a green check or red alert chip below the input. Use `parseEnvRef()` from the same module — don't reimplement the regex.

### Surface references

A model "in use by" a surface (chat / quick_actions / title_generation / speech_to_text) is highlighted in two places:

1. Yellow star (`Star`, `#d9b34a`) next to its row in `ModelList`.
2. `in use by <surface>` badge in the editor header.
3. Warning in the delete confirm.

Use `store.isModelReferencedBySurface(id)` from the settings store — never recompute the mapping in components.

### Reactivity gotchas (Svelte 5)

- Top-level state (`let x = $state(...)`) and effects in this directory follow Svelte 5 runes mode.
- When mirroring a prop into local `$state`, wrap the assignment in `untrack(() => …)` inside `$effect` so writes to the draft don't re-trigger the effect.
- `$effect` that watches an array (e.g., `customEntries`) and fires a debounced save: reference the array first (`customEntries;`) before the timer logic — that's what registers the dependency.

### Window registration reminders

The settings dialog is a separate Tauri window. When adding new windows nearby (e.g., a sub-dialog), remember the project-wide rule: register the HTML entry in `vite.config.ts` `rollupOptions.input` AND in `src-tauri/capabilities/default.json` `"windows"`. See [project root CLAUDE.md](../../../../CLAUDE.md). Prefer not to spawn child dialogs from settings — inline confirms cover all current cases.
