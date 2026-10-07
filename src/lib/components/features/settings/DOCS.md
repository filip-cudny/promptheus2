# Settings UI

Settings dialog window — sidebar navigation + per-section content. Lives in its own Tauri window (`settings-dialog` label, entry `settings-dialog.html` → `src/windows/settings-dialog/App.svelte`).

Design principles are codified in [`migration/settings-ux.md`](../../../../../migration/settings-ux.md). This doc captures the patterns actually applied here so new sections stay consistent.

## Directory Structure

```
settings/
├── SettingsSidebar.svelte         # Left nav, exports SettingsSection union + SIDEBAR_ITEMS
├── SettingsContent.svelte         # Routes activeSection → Section component
├── SettingsSection.svelte         # Shared shell (title + hint + actions / scrollable body / optional footer)
├── SectionModels.svelte           # Models section: list pane + editor pane
├── SectionAppearance.svelte       # Theme toggle
├── SectionHistory.svelte          # History retention window (presets + custom days, confirm before pruning) + storage card (size / reclaimable / Compact database)
├── SectionSpeech.svelte           # STT failure handling: transcription attempts, audio retention hours, keep-audio-on-success
├── SectionSync.svelte             # Git settings sync: state, repo path, branch, last sync, "Sync now" (reads status via `$lib/services/settingsSync`)
├── SectionPromptBase.svelte       # Preferred name field + System / about_you / environment / input_format prompts (tabbed)
├── SectionSurfacePrompts.svelte   # Title generation + STT prompt + STT keyterms (tabbed)
├── SttKeytermsEditor.svelte       # STT keyterms file editor (chip view + raw text fallback)
├── PromptEditor.svelte            # Per-prompt editor (load/edit/autosave + Cmd+S, no Save button)
├── EnvPlaceholdersPopover.svelte  # On-demand popover of {{date}}/{{time}}/... chips for environment.md
├── ModelList.svelte               # Grouped model list, "Add" split-button (text/STT)
├── ModelEditor.svelte             # Single-model form: basic / connection / capabilities / parameters / danger
├── ParametersKnown.svelte         # Sliders + toggles for OpenAI-style known params
├── ParametersCustom.svelte        # Free-form key/type/value rows for arbitrary params
└── EnvRefChip.svelte              # ${ENV_VAR} reference indicator (resolved/missing)
```

Surrounding pieces (not in this dir):

- `src/windows/settings-dialog/App.svelte` — root: mounts sidebar + active section, calls `store.init()` / `store.destroy()`.
- `src/lib/stores/settings.svelte.ts` — single shared rune store (`getSettingsStore()`); subscribes to backend `settings-changed` events.
- `src/lib/services/settings.ts` — typed `invoke()` wrappers for `get_settings`, `update_model`, etc.
- `src/lib/services/settingsDialog.ts` — `openSettingsWindow(section?)` + `checkEnvVar(name)`.

## Conventions

### Adding a new section

1. Add the id to the `SettingsSection` union and `SIDEBAR_ITEMS` array in `SettingsSidebar.svelte`. Set `enabled: true` only when the section actually renders.
2. Create `Section<Name>.svelte` here and branch on it in `SettingsContent.svelte`'s `{#if activeSection === ...}`.
3. Read state from `getSettingsStore()` — never `invoke("get_settings")` directly from a component.
4. Write through `$lib/services/settings.ts` helpers; the store auto-refreshes on the backend's `settings-changed` event.

Disabled sidebar items group under a "Coming soon" heading at the bottom of the nav (rendered smaller and faded), so the enabled section reads as a clean menu. Order in `SIDEBAR_ITEMS` is the user's mental model (General → Models → … → Advanced last), not alphabetical — the sidebar partitions by `enabled` while preserving that order. Active item is marked with a 2px left border in `--accent`, never a filled background, so the accent only signals "you are here" (the same single-accent rule as the conversation window: blue ≡ user/marker, never decoration).

### Persistence — auto-save with debounce

No Save/Cancel buttons. Every input persists on change:

- **Text inputs** (display name, model id, base URL, API key, group, custom param values) — debounced via `scheduleSave(false)`. Debounce is `store.settings.autosave_debounce_ms` (default 1000ms). This is the single source of truth for every autosave debounce in the app — `ModelEditor`, `PromptEditor`, and any future autosave site read it from here. 1000ms is tuned to fire only when the user has actually paused (typical inter-word gap is 250–400ms; 200ms fires mid-sentence).
- **Discrete controls** (segmented buttons, selects, checkboxes, slider toggles) — `scheduleSave(true)` = persist immediately, no debounce.
- **Range sliders and number inputs** — fire on every `oninput` and must debounce. `ParametersKnown` passes `immediate: false` as the third argument of its `onChange` callback for these; the checkbox and the reasoning-effort select pass nothing and stay immediate. Saving a slider drag immediately would write the skill/model file once per animation frame, each write emitting a change event and a full store refresh. The known-parameter override checkbox is the gate: unchecking it sends `null` to drop the override entirely.

Save state is owned by `useSaveTracker()` (`$lib/stores/saveTracker.svelte.ts`) — a small rune-based state machine that owns the debounce timer, the reentrancy guard, the `now`-tick interval driving the saved-stamp expiry, and a single try/catch that captures errors. It exposes `state` (`idle | dirty | saving | saved | error`), `tooltip`, and `error`, plus `scheduleSave(persistFn)`, `flush()`, `runAction(fn)`, `cancel()`, `attachKeyboard()`, `attachBeforeUnload()`, and `destroy()`. `<SaveStatusIndicator {tracker} />` (in `$lib/components/shared/widgets/`) is the pure presentational counterpart — 6 px dot + transient "✓ saved" / "⚠ save failed" stamp + tooltip.

The pattern in `ModelEditor.svelte`:

```ts
const tracker = useSaveTracker({ debounceMs });

onMount(() => {
  tracker.attachKeyboard(window);
  tracker.attachBeforeUnload(window);
});
onDestroy(() => tracker.destroy());

function scheduleSave(immediate: boolean) {
  if (immediate) void tracker.flush(persist);
  else tracker.scheduleSave(persist);
}
```

`tracker.destroy()` must run on unmount to clear timers and detach window listeners; it also fires the pending debounced save before tearing down, because unmount is a normal way to leave an editor (switching sidebar section, or picking another skill through the `{#key}` block) and a 1000 ms debounce is easy to beat.

Saves are **queued, never dropped**. A save arriving while another is in flight waits for it and then runs — the earlier `if (saving) return` guard silently discarded the newer edit while still reporting success, so the last change before a rapid second edit was lost. `dirty` only clears when nothing is queued behind the save that just finished.

Discrete actions that aren't auto-saves (delete, duplicate) go through `tracker.runAction(fn)`, not `flush(fn)`: it persists pending edits first, then runs the action, and its `true`/`false` reflects the action itself. `flush(fn)` only queues `fn` as the next autosave — a concurrent edit can replace it, so an action passed to `flush` may never run while still returning `true`. Callers branch on the boolean to decide whether to advance UI (e.g., switching selection after a successful delete).

The indicator is mounted at the section/editor header — one indicator per save target. Tabbed prompt sections (`SectionPromptBase`, `SectionSurfacePrompts`) keep per-editor dots so the tab the user is on tracks its own document; promoting them to section-level would lie about which document is dirty.

### Draft state, not direct mutation

`ModelEditor` clones the incoming `model` prop into a local `$state` `draft` and edits the draft. A `$effect(() => { const m = model; untrack(() => { draft = structuredClone(m); ... }) })` resets the draft when the parent swaps to a different model. The parent uses `{#key selectedModel.id}` so switching models fully remounts the editor, which combined with `untrack` prevents stale-draft leaks across selections.

`structuredClone` (not spread) — required because `parameters` is nested.

`SkillEditor` mirrors the same shape for the `skill` prop. The `{#key}` remount is a convenience (it resets local UI state like `confirmDelete`), never the mechanism that loads fresh data — the sync effect must do that on its own, see "Reactivity gotchas".

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

`ParametersKnown.svelte` shows each param with a leading checkbox. Unchecked = inherit (sends `null`); checked = enable the slider/input pre-filled with a value. This keeps the form short and makes "I haven't touched this" visually distinct from "I set it to 0".

Pass the `inherited` prop (the surface parameters the editor falls back to — for skills, `surfaces.quick_actions.generation.parameters`) whenever there is a real inheritance chain. It drives two things: an `inherits <value>` caption next to every unchecked param, and the prefill used when the box is checked. Checking a box must not silently change the effective value — prefilling `reasoning_effort` with the first allowed level (`minimal` for OpenAI effort models) while the surface inherits `medium` is a silent downgrade the user never asked for. Static defaults (`temperature: 0.7`, `max_tokens: 4096`) are the fallback when nothing is inherited.

Reasoning effort is a `<select>`, not an `<input list>` + `<datalist>`. Browsers filter datalist suggestions against the field's current text, so a field already holding `medium` offers exactly one suggestion and looks broken; free text also meant every keystroke fired an immediate save of a half-typed level. The row also renders when the value is set but the model reports no effort support (deleted model, capabilities changed) — with a warning — so a stored value can never become invisible and unremovable.

### Missing or unlisted model references

A skill pins a model by id and nothing rewrites that id when the model is deleted, so the editor must handle "the pinned id resolves to nothing". `SkillEditor` renders a synthetic `<option>` for such an id (`<id> — no longer exists` / `— is not a text model`) plus a `warn-text` helper line. Without it the `<select>` has no matching option, silently renders blank, and the skill looks like it inherits while `resolve_quick_action_model` fails at run time (a pinned id never falls back to the Quick Actions model — that's deliberate, an unrunnable skill should say so rather than quietly run on a different model).

The same id also yields no capabilities, so anything capability-gated must degrade to "show the stored value with a warning" rather than disappearing — see the reasoning-effort row in `ParametersKnown`.

The inherit option and an explicit pin to the same model read almost identically, so the option list marks the model that is currently the Quick Actions one.

### Env var references

API key fields accept literal secrets *or* `${VAR_NAME}` references. `EnvRefChip` parses the value, calls `check_env_var` on the backend, and shows a green check or red alert chip below the input. Use `parseEnvRef()` from the same module — don't reimplement the regex.

### Surface references

A model "in use by" a surface (chat / quick_actions / title_generation / speech_to_text) is surfaced in three places, all sharing the same accent-coloured visual language:

1. Accent-coloured caption line in `ModelList` rows: `in use by chat, title generation`, rendered below the model id at `font-size-xs` in `var(--accent)`. Same phrasing as the editor header pill so the two views speak the same vocabulary.
2. Pill badge in the editor header (`in use by chat, title generation`), `--accent-bg-soft` / `--accent` / `--accent-border` tokens. Header uses `align-items: center` so the pill is vertically centred against the heading.
3. Warning sentence in the delete-confirm. This one keeps the `.warn` colour — destructive intent justifies the warning palette.

Sources of truth:

- `store.surfacesByModel` — `Map<modelId, SurfaceKind[]>`, ordered per `SURFACE_ORDER`. Use this when iterating models (e.g. `ModelList`).
- `store.getSurfacesForModel(id)` — `SurfaceKind[]` for a single model. Use this when you have one id (e.g. `ModelEditor` props).
- `formatSurfaceList(surfaces)` from `$lib/constants/surfaces` — the canonical comma-joined human label.

Never recompute the mapping ad hoc; both consumers must read from the store so the list, the editor badge, and the delete confirm stay in lockstep.

The caption/pill replaced an earlier yellow star (in the list) and `--warning`-coloured pill (in the editor). Star semantics ("favorite / user preference") and warning amber ("issue / attention required") both clashed with the actual meaning here ("system binding"). `--warning` is reserved for issue states only.

The list caption replaced a 7 px accent dot before the row name. The dot collided with the auto-save status dot in `SaveStatusIndicator` — same glyph, same colour, two different meanings. Filled circles are reserved for transient state (save indicator); persistent structural facts ("model is bound to surface X") get a textual caption.

### Reactivity gotchas (Svelte 5)

- Top-level state (`let x = $state(...)`) and effects in this directory follow Svelte 5 runes mode.
- `useSaveTracker` is called from `$effect` in half the editors (`PromptEditor`, `SttKeytermsEditor`, `SectionPromptBase`, the `customEntries` and body effects in `SkillEditor`/`ModelEditor`), so **nothing inside the tracker may be a signal the tracker itself reads and writes**. `pending` and `saveTimer` are deliberately plain `let`s: as `$state` they made `scheduleSave` register `saveTimer` as a dependency of the calling effect (`if (saveTimer) clearTimeout(saveTimer)`) and then invalidate it (`saveTimer = setTimeout(…)`) — an infinite effect loop that aborts the flush with `effect_update_depth_exceeded` and leaves controlled `<textarea>`s reverting every keystroke (caret moves, character never appears). The public mutators wrap their synchronous bodies in `untrack()` for the same reason. Reactive `hasPending` comes from a separate write-only `$state` flag; assigning a signal, unlike reading one, creates no dependency.
- When mirroring a prop into local `$state`, read the prop **outside** `untrack` and put everything else — including the `tracker.dirty / saving / hasPending` guard — inside it (`ModelEditor.svelte` is the reference). The prop must be the only dependency, so the effect re-syncs when fresh data arrives and never at any other time. `SkillEditor` had this inverted: `skill` was read inside `untrack` and the tracker flags outside, so the effect re-ran the moment a save finished — restoring the *pre-save* prop over the value just entered, because the store refresh (event → `list_skills_full`) lands later — and never re-ran when the refreshed prop finally arrived. Symptoms were "the value only shows up the second time I select the skill", checkboxes bouncing back after one click, and edits reverting and then being written back to disk by the next autosave.
- `$effect` that watches an array (e.g., `customEntries`) and fires a debounced save: reference the array first (`customEntries;`) before the timer logic — that's what registers the dependency.

### Prompt editor

`PromptEditor.svelte` is the single component for any prompt slot. It loads via `getPrompt(kind)`, autosaves on change with an 800 ms debounce, and supports `Cmd/Ctrl+S` to flush immediately. There is no Save button — the only persistence affordance is `<SaveStatusIndicator>` in the title row (driven by `useSaveTracker` — see "Persistence" above). The file path is hidden behind a small `…` overflow next to the title (custom paths require manual JSON edit).

Only `kind: "environment"` shows the placeholders affordance: a `{ }` icon button in the title row opens an `EnvPlaceholdersPopover` anchored to the button. Clicking a chip inserts the token at the cursor in the textarea and closes the popover. Other prompts get a description that says they are sent verbatim — no placeholder substitution happens for them.

### Section shells & tabbed prompts

`SettingsSection.svelte` is a shared scaffold every section can use: optional title + hint + actions header (with a thin `--border-faint` divider), a scrollable padded body, and an optional sticky footer. Use it for sections that have a "page" feel (`SectionPromptBase`, `SectionSurfacePrompts`); the models section keeps its own list+editor split layout but should still match `SettingsSection`'s padding rhythm where possible.

`SectionPromptBase` and `SectionSurfacePrompts` use a tab strip in the section header — only one `PromptEditor` is mounted at a time and fills the available height. This replaces the old "stacked card-soup" layout where every prompt rendered a full-bordered card stacked vertically. Tabs are uppercase-tracked labels, active tab uses a 2px bottom border in `--accent` (mirrors the sidebar's left-border marker, single-accent rule).

`SectionPromptBase` additionally renders a "Preferred name" identity card above the tabs — a single-field `TextInput` capped at 60 chars (live counter on the right edge, turns `--danger` if `maxlength` is somehow exceeded). It autosaves through `useSaveTracker` + `updateSetting("preferred_name", …)`. The counter and the section's own `SaveStatusIndicator` both belong to this card; the per-editor save dots in the tabs below remain independent.

Editor body width is capped: `--prompt-editor-max-width` (760px for prose-style prompts, 960px for environment with its placeholders popover) keeps long prompts readable. Always prefer wrapping the editor in this max-width rather than letting the textarea fill arbitrary window sizes.

Save flow goes through `$lib/services/prompts.ts` → backend `save_prompt` Tauri command → `ConfigService::write_prompt` → `PromptStore` (atomic tempfile + rename). Backend emits `prompt-changed` Tauri event after each save.

### Window registration reminders

The settings dialog is a separate Tauri window. When adding new windows nearby (e.g., a sub-dialog), remember the project-wide rule: register the HTML entry in `vite.config.ts` `rollupOptions.input` AND in `src-tauri/capabilities/default.json` `"windows"`. See [project root CLAUDE.md](../../../../CLAUDE.md). Prefer not to spawn child dialogs from settings — inline confirms cover all current cases.
