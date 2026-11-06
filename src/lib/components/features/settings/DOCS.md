# Settings UI

Settings dialog window — sidebar navigation + per-section content. Lives in its own Tauri window (`settings-dialog` label, entry `settings-dialog.html` → `src/windows/settings-dialog/App.svelte`).

This doc captures the patterns applied here so new sections stay consistent.

Files whose role does not follow from the name: `SettingsSidebar.svelte` exports the `SettingsSection` union and `SIDEBAR_GROUPS`; `SettingsContent.svelte` routes `activeSection` to a `Section*` component; `SettingsSection.svelte` is the shared section shell, not a section; `SettingField.svelte` is the label + description + error wrapper for every catalog-bound control.

The root is `src/windows/settings-dialog/App.svelte` (calls `store.init()` / `store.destroy()`); state lives in the single rune store `getSettingsStore()` (`$lib/stores/settings.svelte.ts`), which subscribes to the backend `settings-changed` event.

## Conventions

### General section

`SectionGeneral.svelte` holds five fields, each through `SettingField`: "Theme" (button toggling light/dark), "Logo" and "Icons" (each a `SelectDropdown` of Color / Mono, saved with `updateSetting("appearance", { ...current, <field>: value })`), "Launch at startup" and "Show tray icon" (checkboxes saved with `updateSetting(<key>, value)`). The tray icon applies after restart; `SettingField` shows that from its catalog entry.

`SectionSpeech.svelte` holds two selects for the Claude pill delivery, "Claude target" (`claude_target`: webview or Claude Desktop) and "Stop + chat target" (`stop_chat_target`: Chat or Claude); each saves its own field with `updateSpeechToTextConfig`.

### Model creation dialog

"+ model" in `ModelList` opens `CreateModelDialog.svelte` (pattern of `CreateSkillDialog`) with the type preset; nothing is written until the dialog's Create. Fields: Type, Display name, Provider, Model, API key (literal or `${VAR}`, with `EnvRefChip`), optional Base URL. `newModelErrors` (`$lib/utils/newModel.ts`) mirrors the backend `validate` rules; Create is disabled while it returns errors, and an error shows at its field once the field was blurred. A backend rejection shows at the field (`store.fieldError("models[<id>].<field>")`, the id is generated once per dialog) and keeps the dialog open; success closes it and selects the new model.

### Store toggle

`ModelEditor` hides "Store" when `draft.api_mode === "completions"`: the flag has no effect on Chat Completions. The stored value is kept, so switching back to Responses shows it unchanged.

### Tests of pure modules

Pure TS modules (no Svelte runes, no `$lib` imports other than `import type`) live in `src/lib/utils`. Their tests are `tests/*.test.mjs` (`node:test`, `node:assert`), importing the `.ts` file directly through Node type stripping; run `node --test "tests/*.test.mjs"` from the root. There is no test runner dependency; `node --test tests/` does not work on Node 25, which treats the directory as a module.

### Sidebar groups and adding a section

`SIDEBAR_GROUPS` in `SettingsSidebar.svelte` lists three groups, each an ordered array of items: "App" (General, Notifications, History, Sync, Environment), "AI" (Models, Prompt Base, Skills, MCP Servers), "Features" (Chat, Context menu, Speech). There is no disabled item; a section appears in the sidebar only once it renders.

Active item is marked with a 2px left border in `--accent`, never a filled background, so the accent only signals "you are here" (the same single-accent rule as the conversation window: blue ≡ user/marker, never decoration).

1. Add the id to the `SettingsSection` union and an item to a group's array in `SettingsSidebar.svelte`.
2. Create `Section<Name>.svelte` here and branch on it in `SettingsContent.svelte`'s `{#if activeSection === ...}`.
3. Read state from `getSettingsStore()` — never `invoke("get_settings")` from a component.
4. Write through `$lib/services/settings.ts` helpers; the store auto-refreshes on the backend's `settings-changed` event.

### SettingField

Every control bound to a settings key is wrapped in `<SettingField key="<catalog path>">` (`itemId` for list items). It takes label and description from the catalog entry (`getSettingsCatalogStore()` in `$lib/stores/settingsCatalog.svelte.ts`, loaded once per window), marks `after_restart` entries, renders the backend field error whose concrete key maps to `key`, and sets `id="setting-<key>"`. A new key needs a catalog entry in `services/config/catalog.rs`; without one the field shows the key as its label and logs a warning. Label and help text are never written in the component.

### Agent panel

The "Ask agent" row (lucide `Sparkles`, no accent colour) directly under the search in `SettingsSidebar` toggles `agent/AgentPanel.svelte` (380 px, `PANEL_WIDTH` in `services/settings_agent/panel.rs`). The panel header has `Sparkles` and the title on the left, "New conversation" (`SquarePen`, disabled while the agent runs) and "Close" (`X`) on the right.

Layout depends only on the width of `.dialog-body` (a CSS container named `settings-body`; `.with-panel` while the panel is open), not on the panel mode:

- At least 1340 px (sidebar 220 + content 740 + panel 380): sidebar | content (`flex: 0 1 var(--column-width)`, 1040 px) | panel (`flex: 1 1 380px`, `min-width: 380px`) in one row, so no empty band is left between a width-limited editor and the panel.
- Narrower: the panel overlays the content's right edge at 380 px.

`settings_agent_panel(open)` still returns a mode (`widened` or `overlay`) that drives only the window widening; the UI does not read it:

- `widened`: the window grew by 380 px.
- `overlay`: the window did not grow. Chosen when the window is maximized, the monitor work area has no room for width + 380, or the size did not change within ~300 ms of the resize (tiled window; GNOME Wayland ignores the resize).

Geometry rule: the backend keeps the added width (0 in overlay mode); `save_geometry` subtracts it for `settings-dialog` (`geometry_without_panel`), so the saved width never includes the panel. Closing the panel shrinks the window by the added width; closing the window resets it.

State is `getSettingsAgent()` (`$lib/stores/settingsAgent.svelte.ts`): messages (user text, assistant text from `chunk` events, tool calls as stored `ToolCall`s: `tool_call_start` pushes the call, `tool_call_done` sets `result`, `error`, `status` and `completed_at`, and when `send` ends calls still `in_progress` or `pending` become `cancelled`; `AgentMessages` splits the text at the `{{tool_call:<id>}}` markers with `$lib/utils/toolCallMarkers.ts` (shared with `AssistantBubble`) and renders each run of markers as a `ToolCallGroup` at its position, skipping ids without a stored call), `running`, `error`, `send`/`cancel`/`reset` over `$lib/services/settingsAgent.ts`. It calls `settings_agent_reset` once on load; the conversation lives in the module state, so it survives closing the panel and ends with the window. `AgentComposer` has a `beforeSend` snippet slot and a mic button (`Mic`, disabled while the agent runs): a click calls `dictate()` (`start_settings_agent_dictation`; a failure lands in `error`), and the `settings-agent-dictation { text }` event appends the transcript to the textarea (space-separated when not empty) and focuses it; nothing is sent. See `src-tauri/src/services/speech/DOCS.md` "Settings agent destination".

Links: the assistant writes `[label](setting:<key>)`. `$lib/utils/settingLinks.ts` (`parseSettingLink`, `showSettingLinks`) relabels them "show" before markdown rendering; a click on `setting:` calls `navigateTo(key)`, any other link opens through `openUrl`.

### Missing environment variables

`MissingEnvList.svelte` sits above sidebar and content in `src/windows/settings-dialog/App.svelte` and renders only when `getMissingEnvRefs()` (backend `get_missing_env_refs`) returns names: each `${VAR}` without a value on this machine with the labels of where it is used, plus an "Add" button that opens `SecretField` inline under the name (no agent involved); closing it refreshes the list. It refreshes on `settings-changed` (`onSettingsChanged`) and window focus. The same set drives the startup/sync toast (`src-tauri/src/services/DOCS.md`).

### Environment section

`SectionEnvironment` (sidebar "Environment", group App) lists `getEnvVars()` (`get_env_vars` in `$lib/services/settings.ts`): per variable the name, a "secret" marker, state "set" / "missing" (colours of `EnvRefChip`) and usage labels or "not used". It refreshes on mount, `settings-changed` and window focus. Every change goes through `applyEnvVarOperation(operation)` (`apply_env_var_operation`, operations `add_env_var`, `update_env_var`, `rename_env_var`, `delete_env_var`; each writes a change-log entry, so Undo is in `ChangeHistory`) and then reloads the list; a rejection shows its message at the row (at the add form for an add; the `row N: ` prefix is cut; delete of a used name lists the usages) and changes nothing locally. Names are checked client-side with `^[A-Za-z_][A-Za-z0-9_]*$` before sending.

Per row: description (debounced by `autosave_debounce_ms`, pending edits flushed on rename and on unmount) and the "Secret" checkbox (immediate) both send `update_env_var` with both fields; "Set value" / "Change value" opens `SecretField` with `exists` = set and `masked` = the secret flag; "Rename" opens an inline name input (Enter confirms, Escape cancels); "Delete" asks with the inline two-step confirm. "Add variable" (secret on by default) opens the value field under the new row. Values are never held by the section.

### Proposal card

`agent/ProposalCard.svelte` renders the store's `proposal` (event `settings-agent-proposal`, one card; a new proposal replaces it, a refresh keeps the checkbox state because the card is keyed by `proposal_id`) after the secret requests in `AgentMessages`. `ProposalRow` per row: checkbox (checked by default), label, each effect as before → after (setting effects) or a line diff (`lineDiff` in `$lib/utils/lineDiff.ts`, file effects: skill and prompt files), the test result (`OK · <ms> ms` or the exact error; skill output and sample input in collapsed "Output" and "Sample input"; `sample_input` comes from the proposal payload, `null` for other rows), "Test" (only for model, skill and MCP rows; `settings_agent_test_row`) and "show" (`navigateTo` with the setting's concrete key; skill files navigate with `skills[<slug>]`, which `navigateTo` resolves without a catalog entry by switching to Skills and setting `selectedItem`, which `SectionSkills` keeps as the requested slug until the skills store contains it, then selects; prompt files switch to Prompt Base). A stale row shows "changed since proposed"; a `stale_rows` result of Apply marks those rows stale in the held proposal; "Refresh" (`settings_agent_refresh`) is offered and Apply stays disabled until none is stale. Apply sends one boolean per operation index; `stale_rows` or `error` (dependency errors name the dependent rows) show on the card and nothing is written. Success shows "Applied"; the entry id is in `proposalOutcome` of the store for the history view. Reject closes the card.

### Secret field

`SecretField.svelte` (props `name`, `purpose`, `exists`, `masked` (default `true`), `onclose(outcome)`): masked input, reveal toggle (`Eye`/`EyeOff`; for an existing value the reveal loads it with `get_secret_value`), Save (`save_secret`, writes at once as its own change, independent of any card) and Cancel. `masked: false` renders a plain text input without the reveal toggle and loads an existing value on mount (for variables that are not secrets, e.g. a path). `onclose` gets `saved` or `cancelled`; the value lives only in the component state, never in a store, log or event. For agent requests the outcome goes to `settings_agent_secret_result`; the backend turns `cancelled` into `exists` when the variable already has a value.

### Persistence — auto-save with debounce

No Save/Cancel buttons. Every input persists on change:

- **Text inputs** (display name, model id, base URL, API key, group, custom param values) — debounced via `scheduleSave(false)`. Debounce is `store.settings.autosave_debounce_ms` (each editor gets it as a `debounceMs` option or prop, falling back to `?? 1000`); every autosave site reads it from here. 1000ms is tuned to fire only when the user has actually paused (typical inter-word gap is 250–400ms; 200ms fires mid-sentence).
- **Discrete controls** (segmented buttons, selects, checkboxes, slider toggles) — `scheduleSave(true)` = persist immediately, no debounce.
- **Range sliders and number inputs** — fire on every `oninput` and must debounce. `ParametersKnown` passes `immediate: false` as the third argument of its `onChange` callback for these; the checkbox and the reasoning-effort select pass nothing and stay immediate. Saving a slider drag immediately would write the skill/model file once per animation frame, each write emitting a change event and a full store refresh. The known-parameter override checkbox is the gate: unchecking it sends `null` to drop the override entirely.

Save state is owned by `useSaveTracker()` (`$lib/stores/saveTracker.svelte.ts`) — a small rune-based state machine that owns the debounce timer, the reentrancy guard, the `now`-tick interval driving the saved-stamp expiry, and a single try/catch that captures errors. It exposes `state` (`idle | dirty | saving | saved | error`), `tooltip`, and `error`, plus `scheduleSave(persistFn)`, `flush()`, `runAction(fn)`, `cancel()`, `attachKeyboard()`, `attachBeforeUnload()`, and `destroy()`. `<SaveStatusIndicator {tracker} />` (in `$lib/components/shared/widgets/`) is the pure presentational counterpart — 6 px dot + transient "✓ saved" / "⚠ save failed" stamp + tooltip. Clicking the "save failed" stamp copies the error and swaps the stamp to an uncoloured "✓ copied" for `COPY_FEEDBACK_MS`.

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

Discrete actions that aren't auto-saves (delete, duplicate) should go through `tracker.runAction(fn)`, not `flush(fn)` (`SkillEditor` does; `ModelEditor` still passes its delete and duplicate to `flush`): it persists pending edits first, then runs the action, and its `true`/`false` reflects the action itself. `flush(fn)` only queues `fn` as the next autosave — a concurrent edit can replace it, so an action passed to `flush` may never run while still returning `true`. Callers branch on the boolean to decide whether to advance UI (e.g., switching selection after a successful delete).

The indicator is mounted at the section/editor header — one indicator per save target. The tabbed prompt section (`SectionPromptBase`) keeps per-editor dots so the tab the user is on tracks its own document; promoting them to section-level would lie about which document is dirty.

### Draft state, not direct mutation

`ModelEditor` clones the incoming `model` prop into a local `$state` `draft` and edits the draft. A `$effect(() => { const m = model; untrack(() => { draft = structuredClone(m); ... }) })` resets the draft when the parent swaps to a different model. The parent uses `{#key selectedModel.id}` so switching models fully remounts the editor, which combined with `untrack` prevents stale-draft leaks across selections.

`structuredClone` (not spread) — required because `parameters` is nested.

`SkillEditor` mirrors the same shape for the `skill` prop. The `{#key}` remount is a convenience (it resets local UI state like `confirmDelete`), never the mechanism that loads fresh data — the sync effect must do that on its own, see "Reactivity gotchas".

### Validation

Client-side: `ModelEditor.validate()` populates `validationErrors: Record<string, string>`; `persist()` early-returns on invalid state, with a red border and field error.

Backend: `ConfigService` rejects an invalid write with `<message> (setting: <concrete key>)`. Every write in `services/settings.ts` goes through `invokeWrite`, which parses the error (`parseSettingError`) and calls `store.reportWriteError`: a key lands in `store.fieldErrors` (read with `store.fieldError(key)`), an error without a key becomes `store.toastError`, shown by `SettingsErrorToast` (auto-dismiss after 6 s). A successful write clears `fieldErrors`. `ModelEditor.errorFor(field)` shows `validationErrors[field]` or the backend error at `models[<id>].<field>`; a new field editor that wants backend errors reads `store.fieldError(<concrete key>)` the same way.

Custom param errors live in `customErrors` keyed by `entry.id` (the parse function in `ParametersCustom.svelte`'s module script returns `{ extra, errors }`).

### Sections as cards

Inside a section, group related fields in `<section class="card">` with an uppercase `<h3>` heading (e.g., "Basic", "Connection", "Capabilities", "Parameters", "Danger zone"). Card styles live in `ModelEditor.svelte`'s scoped style — copy them when adding a new section to keep visual rhythm consistent.

### Danger zone

`DangerZone.svelte` is the last card of `ModelEditor` and `SkillEditor`: red border (`var(--danger-border)`), separated visually. Props: `noun`, `name` (shown in the confirm), `compact`, `warning` snippet, `onDuplicate`, `onDelete`, optional `onExport`. Destructive actions use a two-step inline confirm — never an OS dialog or a modal-on-modal.

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

### Skill "Offered in"

`SkillEditor` shows `offered_in` as two checkboxes, "Context menu" and "Recording widget" (`pill`). Both checked sends `offered_in: null` (no `offered_in` line in the file, offered everywhere); both unchecked sends `[]` (`offered_in: []`, offered only in the chat); one checked sends that value. `persist` always sends the field, because `update_skill` writes exactly the frontmatter it receives — omitting it would erase a restriction on every edit of another field. The chat `/skill` autocomplete ignores the field.

### Missing or unlisted model references

A skill pins a model by id and nothing rewrites that id when the model is deleted, so the editor must handle "the pinned id resolves to nothing". `SkillEditor` renders a synthetic `<option>` for such an id (`<id> — no longer exists` / `— is not a text model`) plus a `warn-text` helper line. Without it the `<select>` has no matching option, silently renders blank, and the skill looks like it inherits while `resolve_quick_action_model` fails at run time (a pinned id never falls back to the Quick Actions model — that's deliberate, an unrunnable skill should say so rather than quietly run on a different model).

The same id also yields no capabilities, so anything capability-gated must degrade to "show the stored value with a warning" rather than disappearing — see the reasoning-effort row in `ParametersKnown`.

The inherit option and an explicit pin to the same model read almost identically, so the option list marks the model that is currently the Quick Actions one.

### Env var references

API key fields accept literal secrets *or* `${VAR_NAME}` references. `ModelEditor` parses the value with `parseEnvRef()` (exported by `EnvRefChip.svelte` — don't reimplement the regex), calls `checkEnvVar` (`$lib/services/settingsDialog`, backend `check_env_var`) and passes the result to `EnvRefChip`, which shows a green check or red alert chip below the input.

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

`PromptEditor.svelte` is the single component for any prompt slot. It loads via `getPrompt(kind)`, autosaves on change with the `autosave_debounce_ms` debounce, and supports `Cmd/Ctrl+S` to flush immediately. There is no Save button — the only persistence affordance is `<SaveStatusIndicator>` in the title row (driven by `useSaveTracker` — see "Persistence" above). The file path is hidden behind a `FilePathReveal` (`…`) next to the title (custom paths require manual JSON edit).

Only `kind: "environment"` shows the placeholders affordance: a `{ }` icon button in the title row opens an `EnvPlaceholdersPopover` anchored to the button. Clicking a chip calls `HighlightedTextarea.insert(token)` and closes the popover. The editor icon buttons use the global `.editor-icon-btn` class (`styles/global.css`). Other prompts get a description that says they are sent verbatim — no placeholder substitution happens for them.

A `PromptEditor` or `SttKeytermsEditor` inside a fixed-height wrapper (Chat 280px, Speech 320px) must fit in it: the Chat and Speech `PromptEditor`s and the keyterms raw pane pass `minHeight={0}` to `HighlightedTextarea` (its default 320px min-height plus the editor header is taller than the wrapper), and the keyterms chip pane has no min-height, so it shrinks and scrolls. Content taller than the wrapper is painted over the next field.

### Section shells & tabbed prompts

`SettingsSection.svelte` is a shared scaffold every section can use: optional title + hint header (with a thin `--border-faint` divider) and a scrollable padded body. Use it for sections that have a "page" feel (`SectionPromptBase`, `SectionChat`, `SectionSpeech`); the models section keeps its own list+editor split layout but should still match `SettingsSection`'s padding rhythm where possible.

Content column: every section sits in one column of the same width, `--column-width` (1040 px), centered in `.content`, so the column's left edge does not move between sections and on a wide window the free width is split evenly left and right. Inside the column each section keeps its own form widths, left-aligned; never center a section by its own width. `SettingsContent` defines `--column-gutter` (`max(0px, (100% - var(--column-width)) / 2)`, a percentage, so it resolves against the containing block where it is used and is correct only on an element whose containing block spans the whole `.content` width). Applied as `.head` `margin-inline` (the header divider spans the column, not the window) and added to `.body` `padding-inline` in `SettingsSection`, and in `SectionModels` as the model list's `margin-left` and the editor pane's `padding-right`. The scroll containers (`.body`, the models editor pane) keep the full width, so their scrollbar stays at the right edge of the content area; only the Skills editor pane, a scroller inside `.body`, has it at the column's right edge. With the agent panel beside the content (`.with-panel`, at least 1340 px) `.content` is at most `--column-width` wide, so the gutter is 0; with the panel overlaid the column stays centered under it.

`SectionPromptBase` uses a tab strip in the section header — only one `PromptEditor` is mounted at a time and fills the available height. This replaces the old "stacked card-soup" layout where every prompt rendered a full-bordered card stacked vertically. Tabs are uppercase-tracked labels, active tab uses a 2px bottom border in `--accent` (mirrors the sidebar's left-border marker, single-accent rule).

`SectionPromptBase` additionally renders a "Preferred name" identity card above the tabs — a single-field `TextInput` capped at 60 chars (live counter on the right edge, turns `--danger` if `maxlength` is somehow exceeded). It autosaves through `useSaveTracker` + `updateSetting("preferred_name", …)`. The counter and the section's own `SaveStatusIndicator` both belong to this card; the per-editor save dots in the tabs below remain independent.

Editor body width is capped (760px for prose-style prompts, 960px for environment with its placeholders popover; hardcoded in the section styles) to keep long prompts readable; don't let the textarea fill arbitrary window sizes.

Save flow goes through `$lib/services/prompts.ts` → backend `save_prompt` Tauri command → `ConfigService::write_prompt` → `PromptStore` (atomic tempfile + rename). Backend emits `prompt-changed` Tauri event after each save.

### Chat and Speech sections

`SectionChat` shows the default chat model, reasoning effort (as the model editor's effort select: "None" first, all levels when the model does not advertise effort, a saved value the model does not accept shown as "not accepted by this model"; plus "Default" = `null`), MCP tools as checkboxes of qualified `server.tool` names (from `list_mcp_tools`; empty state when none), the title generation model and prompt (`PromptEditor` kind `title_generation`), and `WebviewProvidersEditor`.

`SectionSpeech` shows STT model, language (empty = `null`), `no_verbatim`, the STT prompt (`PromptEditor` kind `speech_to_text`), keyterms (`SttKeytermsEditor`) and the `speech_to_text_toggle` shortcut, then the recording and failure-handling cards.

`ShortcutField` (prop `action`) reads the binding of `action` in the keymap group of the current OS (`os==macos` when `isMac`, else `os==linux`). "Change" enters capture: the next keydown with at least one modifier and one key becomes the string `cmd|super`, `ctrl`, `alt`, `shift` (in that order, `cmd` for Meta on macOS, `super` elsewhere) plus the key (`a`, `1`, `f1`…`f24`, `space`, `tab`, `enter`, `backspace`, `delete`, `up`, `down`, `left`, `right`, `home`, `end`, `page_up`, `page_down`), joined by `+`; Escape cancels; "Clear" sends `null`. The capture handler calls `stopPropagation()`. It saves with `setShortcut`; the field sits in `SettingField key="keymaps.<action>"`, so a collision error from the backend shows at the field.

`WebviewProvidersEditor` edits the whole `webview_providers` list (name and URL rows, add, remove) and saves it with `updateSetting` — text debounced, remove immediate. Existing ids are kept; a new row gets its id on first save: the name lowercased with characters outside `[a-z0-9_-]` replaced by `-`, suffixed `-2`, `-3`… when taken. Rows without a name are not saved. The list is seeded from the store on mount and not re-synced from later external changes.

`SectionContextMenu` shows the quick actions model and reasoning effort (`effortOptions` from `$lib/utils/effortOptions.ts`, shared with `SectionChat`), `menu_section_order`, `recent_apps_count` (`NumberInput`, min 0, saved after `autosave_debounce_ms`) and `ShortcutField` for `open_context_menu`, `execute_active_prompt`, `set_context_value`, `append_context_value`, `clear_context`. The order list shows the ids present in the setting, named (`chat` Chat; `skills`, `prompts`, `SkillProvider` Skills; `models` Models; `settings` Settings; `speech`, `SpeechMenuProvider` Speech; `lastInteraction`, `LastInteractionMenuProvider` Last interaction; `context`, `ContextMenuProvider` Context; an unknown id as is). HTML5 drag-and-drop with a grip handle moves an item; a drop saves the whole list with every id exactly as stored, so legacy aliases are never rewritten to canonical ids and no section is added or removed.

### MCP Servers section

`SectionMcpServers` edits the whole `mcp_servers` map: list on the left with a state chip per server (`running`, `error`, `starts after restart` for a configured server the registry did not try, `removed after restart` for one it ran that is no longer configured; states from `listMcpServers()` in `$lib/services/mcp.ts`, read once on mount), editor on the right (name, command, args one per line, env rows with `EnvRefChip` for `${VAR}` values, timeout, tool timeout rows). A failed server shows its start error in full, a running one its tools. Rows are a local draft seeded on mount; every edit saves the whole map through `updateSetting("mcp_servers", map)` (text debounced via `useSaveTracker`, delete immediate after an inline two-step confirm). Duplicate names are not saved and show an error at the name. Backend errors show at `mcp_servers.<name>` (name) and in `SettingField` for the catalog keys; `SettingField` gets no `itemId` there because its `itemId` match expects `[id]` keys and these are dotted. Changes apply after restart.

### Search and navigation

`SettingsSearch` (top of the sidebar) matches the query words as prefixes of words of an entry's catalog label or description (`searchCatalog` in `$lib/utils/settingsSearch.ts`; label matches first, max 20). Arrow keys move, Enter or click navigates and clears the query, Escape clears; its keydown handler stops propagation.

`navigateTo(key)` of `getSettingsNavigation()` (`$lib/stores/settingsNavigation.svelte.ts`) is the single entry point for jumps, also for the settings agent's links. `key` is a catalog or concrete path (`models[<id>].api_key`, `mcp_servers.<name>.env`). It switches `activeSection` (owned by that store), sets `selectedItem` when the concrete path names a list item (`SectionModels` and `SectionMcpServers` select it), scrolls `setting-<catalog path>` into view and sets `highlighted` for 2000 ms; `SettingField` draws an accent outline while its catalog path matches.

### Change history

`ChangeHistory` is a card at the bottom of `SectionHistory`: the settings agent's change-log entries of all hosts, newest first (`listChangeLog` in `$lib/services/changeLog.ts`). Each entry shows a title (`entryTitle` in `$lib/utils/changeLog.ts`: row labels for a card, `Secret <name>`, `Undo of <time of the undone entry>`), local time, host, kind, the request, and a state badge `applied` / `undone`. Expanding it shows each row's effects as before → after (file effects as a line diff) and test results; a secret entry shows only the variable name. Undo calls `undoChange(id, false)`: `conflicts` shows an inline list of targets with current vs expected value and "Undo anyway" (`force: true`); `skipped` shows "Secret backup is on <host>"; `undone` reloads the list. The list reloads on mount, on `change-log-changed` and on window focus.

### Window registration reminders

The settings dialog is a separate Tauri window. When adding new windows nearby (e.g., a sub-dialog), register the HTML entry in `vite.config.ts` `rollupOptions.input` AND the label in `src-tauri/capabilities/default.json`: `"windows"` for windows, `"webviews"` for child webviews (`conversation-dialog`, `shell-toolbar`). See [project root CLAUDE.md](../../../../../CLAUDE.md). Prefer not to spawn child dialogs from settings — inline confirms cover all current cases.
