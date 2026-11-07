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

"+ model" in `ModelList` opens `CreateModelDialog.svelte` (pattern of `CreateSkillDialog`) with the type preset; nothing is written until the dialog's Create. Fields: Type, Display name, Provider, Model, API key (`EnvVarPicker`, no literal input), optional Base URL. `newModelErrors` (`$lib/utils/newModel.ts`) mirrors the backend `validate` rules; Create is disabled while it returns errors, and an error shows at its field once the field was blurred (the API key: once a value was picked). A backend rejection shows at the field (`store.fieldError("models[<id>].<field>")`, the id is generated once per dialog) and keeps the dialog open; success closes it and selects the new model. A "New…" in the API key picker writes the variable to the list (and its value) at once, before Create; only the model waits for Create.

### Store toggle

`ModelEditor` hides "Store" when `draft.api_mode === "completions"`: the flag has no effect on Chat Completions. The stored value is kept, so switching back to Responses shows it unchanged.

### Models section — Assignments

`ModelList` pins an "Assignments" row (lucide `ArrowLeftRight`) above the groups and the empty state; it is always shown. `SectionModels` selects either Assignments (`selectedId === null`, the default on first open and after a delete) or a model id; an id that is no longer in `store.models` falls back to Assignments, and a `navigateTo` key naming a model (`models[<id>]…`) still selects that model. The editor pane shows `ModelAssignments` for Assignments, else `ModelEditor`. `SectionModels` calls `getSkillsStore().init()` on mount (idempotent), so skill pins are known when Settings opens straight into Models.

`ModelAssignments` has three cards: "Text features" (Chat, Quick actions, Title generation, Settings agent: model select, reasoning effort select, "Open in <section> ›"), "Speech" (Speech to text, STT model select) and "Pinned in skills" (rendered only when a skill in `getSkillsStore().full` has a non-empty `model`; read-only rows with the model name or the stored id, link "Open skill ›" → `navigateTo("skills[<name>]")`). Each surface row has the element id `assignment-<surface>`. Model selects write like the sections do (`updateSurfaceModel`, `setSpeechToTextModel`); the empty option is "Not set" for Chat and Speech, `sameAsChatLabel` for the others. The effort select of Chat, Quick actions and Settings agent takes its options from the capabilities of the effective model (`chat` id, `quickActionsModelId`, `agentModelId`); Title generation and Speech have no effort cell.

"Open in" calls `navigateTo` with `surfaces.chat.generation.model_id` (Chat), `surfaces.quick_actions.generation.model_id` (Context menu), `surfaces.title_generation.generation.model_id` and `surfaces.settings_agent.generation.model_id` (Chat), `surfaces.speech_to_text.model_id` (Speech). The view uses no `SettingField`: `navigateTo` finds a field by `setting-<catalog path>`, which must stay unique to the field's own section.

`SURFACE_LABELS` (`$lib/constants/surfaces`) is sentence case (`Chat`, `Quick actions`, …): these are the row labels here and the texts of the surface tags (`modelUseLabel`).

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

The "Ask agent" row (lucide `Sparkles`, no accent colour) directly under the search in `SettingsSidebar` toggles `agent/AgentPanel.svelte`. The panel header has `Sparkles`, the title and the model the agent uses (muted, one line with ellipsis; effective id from `$lib/utils/agentModel.ts`: the settings agent model, else the chat model; "No model" when unset or unknown; nothing until settings load) on the left, "New conversation" (`SquarePen`, disabled while the agent runs) and "Close" (`X`) on the right.

Layout depends only on the width of `.dialog-body` (measured with `bind:clientWidth` in `App.svelte`) and the saved panel width, not on the panel mode. `agentPanelLayout(containerWidth, savedWidth)` in `$lib/utils/agentPanelLayout.ts` (tested in `tests/agentPanelLayout.test.mjs`) returns `{ docked, width }`:

- `containerWidth >= CONTENT_MIN_WIDTH + 300` (`PANEL_MIN_WIDTH`): docked. sidebar | content (`flex: 1`) | panel in one row, panel `position: relative` at `min(max(saved, 300), containerWidth − CONTENT_MIN_WIDTH)`, no shadow.
- Narrower: the panel overlays the content's right edge (`position: absolute`, shadow) at `min(max(saved, 300), containerWidth)`.

`CONTENT_MIN_WIDTH` (860 px, sidebar included) is the one constant for the narrowest width at which no settings layout clips or scrolls horizontally. `App.svelte` sets it on `.dialog-body` as `--content-min-width`; the docked panel's `max-width: calc(100% - var(--content-min-width))` keeps the content at that minimum between a container resize and the next measurement. Derived from the CSS widths (box-sizing is `border-box`), to be confirmed in a live run:

- Models: sidebar 220 + model list 280 + editor pane 342 = 842. The pane is padding 24 + 24, card border 1 + 1, card padding 16 + 16 and a card body of 260 px for the widest row that does not wrap (a parameter row: checkbox, label, "inherits <value>" caption, number input). 860 leaves 18 px.
- MCP servers: sidebar 220 + section padding 16 + 16 + server list 220 + gap 16 + editor 320 = 808. An env row is name input 38 % + gap 6 + value cell + gap 6 + remove button 22; the value cell holds the `NAME · missing` chip (about 110) with the × (14), `InsertVariableButton` (26) and gaps (12), at least 160, so `0.62 × editor − 34 >= 160` gives 320.
- Card rows (`ProposalRow`): a column of wrapping flex rows with `min-width: 0` fields, so they fit the 300 px minimum panel width; the diff and output blocks scroll inside their own `<pre>` (`overflow: auto`), deliberately.

The window minimum is 800 px, so the panel always overlays there.

Drag handle: a 6 px strip on the panel's left edge (`col-resize`, pointer capture). While dragging, the store's `panelWidth` follows `panelRight − pointerX`, clamped by `clampPanelWidth` (`[300, containerWidth − CONTENT_MIN_WIDTH]` docked, `[300, containerWidth]` overlaid); on pointer up the store saves the integer px with `setUiState("settings-dialog.agent_panel_width", px)` (`ui_state.json`, local to the machine). `setPanelOpen(true)` reads it with `getUiState` (`DEFAULT_PANEL_WIDTH` 380 when absent or not a number). A failed read or write lands in the store's `error`. There is no keyboard resize.

`settings_agent_panel(open)` still returns a mode (`widened` or `overlay`) that drives only the window widening; the UI does not read it:

- `widened`: the window grew by the saved panel width (the same key, read by `services/settings_agent/panel.rs` from the managed `UiStateService`; absent or not a number: 380; at least 300).
- `overlay`: the window did not grow. Chosen when the window is maximized, the monitor work area has no room for width + the panel width, or the size did not change within ~300 ms of the resize (tiled window; GNOME Wayland ignores the resize).

Geometry rule: the backend keeps the added width (0 in overlay mode); `save_geometry` subtracts it for `settings-dialog` (`geometry_without_panel`), so the saved width never includes the panel. Closing the panel shrinks the window by the added width; closing the window resets it.

State is `getSettingsAgent()` (`$lib/stores/settingsAgent.svelte.ts`): messages (user text; the assistant reply state and the reduction of stream events live in `$lib/utils/agentReply.ts`: text from `chunk` events, tool calls as stored `ToolCall`s: `tool_call_start` appends the call's `{{tool_call:<id>}}` marker to the text (as main chat, so a call shows when it starts, not with the next round's text) and pushes the call, `tool_call_done` sets `result`, `error`, `status` and `completed_at`; thinking of each iteration is kept, a `tool_call_start` closes the iteration, and `AgentMessages` shows it in `ThinkingBlock` above the reply (main chat keeps only the last iteration's thinking); when `send` ends it clears the thinking-active flag and calls still `in_progress` or `pending` become `cancelled`; `AgentMessages` splits the text at the `{{tool_call:<id>}}` markers with `$lib/utils/toolCallMarkers.ts` (shared with `AssistantBubble`) and renders each run of markers as a `ToolCallGroup` at its position, skipping ids without a stored call; below the last reply `ProcessingIndicator` ("Generating") shows while `showsProcessing(reply, running)` holds: the agent runs, the reply is not thinking, and either it has no text, no thinking and no call `in_progress`/`pending`, or every call is `completed`, `failed` or `cancelled` — the rule of `AssistantBubble`), `running`, `error`, `send`/`cancel`/`reset` over `$lib/services/settingsAgent.ts`. It calls `settings_agent_reset` once on load; the conversation lives in the module state, so it survives closing the panel and ends with the window. `AgentComposer` has a `beforeSend` snippet slot and a mic button (`Mic`, disabled while the agent runs): a click calls `dictate()` (`start_settings_agent_dictation`; a failure lands in `error`), and the `settings-agent-dictation { text }` event appends the transcript to the textarea (space-separated when not empty) and focuses it; nothing is sent. See `src-tauri/src/services/speech/DOCS.md` "Settings agent destination".

Links: the assistant writes `[label](setting:<key>)`, or `[label](setting:section:<id>)` for a whole section (`<id>` one of the 12 section ids; `parseSectionLink` in `$lib/utils/settingLinks.ts`). `$lib/utils/settingLinks.ts` (`parseSettingLink`, `showSettingLinks`) relabels them "show" before markdown rendering; a click on `setting:` calls `navigateTo(key)`, any other link opens through `openUrl`.

### Missing environment variables

`MissingEnvList.svelte` sits above sidebar and content in `src/windows/settings-dialog/App.svelte` and renders only when `getMissingEnvRefs()` (backend `get_missing_env_refs`) returns names: each `${VAR}` without a value on this machine with the labels of where it is used, plus an "Add" button that opens `SecretField` inline under the name (no agent involved); closing it refreshes the list. It refreshes on `settings-changed` (`onSettingsChanged`) and window focus. The same set drives the startup/sync toast (`src-tauri/src/services/DOCS.md`).

### Environment section

`SectionEnvironment` (sidebar "Environment", group App) lists `getEnvVars()` (`get_env_vars` in `$lib/services/settings.ts`) in two cards, "Secrets" first, then "Variables", split by the `secret` flag with `groupByKind` (`$lib/utils/envVarOptions.ts`, each group sorted by name). A card shows its rows, a muted "No secrets." / "No variables." line when the list is loaded and the group is empty, then its own add form. The hint reads "Pick these in models and MCP servers. Names and descriptions sync; values stay on this machine." It refreshes on mount, `settings-changed` and window focus. Every change goes through `applyEnvVarOperation(operation)` (`apply_env_var_operation`, operations `add_env_var`, `update_env_var`, `rename_env_var`, `delete_env_var`; each writes a change-log entry, so Undo is in `ChangeHistory`) and then reloads the list; a rejection shows its message at the row (at the add form of its card for an add; the `row N: ` prefix is cut; delete of a used name lists the usages) and changes nothing locally. Names are checked client-side with `^[A-Za-z_][A-Za-z0-9_]*$` before sending.

Add form per card: name, description, "Add" (Enter in an input adds); `secret` is `true` in the Secrets card and `false` in the Variables card. After an add the value field opens under the new row.

Row header: name, state "set" / "missing" (`--success` check / `--danger` alert), "Set value" / "Change value" (opens `SecretField` with `exists` = set and `masked` = the secret flag, so masked in Secrets and plain in Variables), "Edit" and "Delete" (inline two-step confirm). Row body: the description when non-empty, then `Used in: <labels>` or "not used". Values are never held by the section.

"Edit" opens a panel in the row, one row at a time: name and description inputs prefilled with the stored values, "Save", "Cancel" and "Move to Variables" (Secrets row) / "Move to Secrets" (Variables row). Enter saves, Escape cancels (`stopPropagation()`). The edit state is keyed by the stored name, because rows are keyed by it. Save runs in this order: a changed name that fails the pattern shows the name error at the row and sends nothing; a changed description sends `update_env_var` (old name, new description, stored `secret`); a changed name then sends `rename_env_var`. When both change, the change log gets two entries. A rejection shows at the row and stops: after a failed description update the panel stays open; after a failed rename the row, its open panel with the typed name and the already saved description stay. Success closes the panel and clears an open value field of the old name. Nothing changed: the panel closes without an operation. "Move" sends `update_env_var` with the stored name and description and the flipped `secret` at once; the row then renders in the other card.

### Proposal card

`agent/ProposalCard.svelte` renders the store's `proposal` (event `settings-agent-proposal`, one card; a new proposal replaces it, a refresh keeps the checkbox state and typed values because the card is keyed by `proposal_id`) after the messages in `AgentMessages`. `ProposalRow` per row: checkbox (checked by default), label, each effect as before → after (setting effects) or a line diff (`lineDiff` in `$lib/utils/lineDiff.ts`, file effects: skill and prompt files), the test result (`OK · <ms> ms` or the exact error; skill output and sample input in collapsed "Output" and "Sample input"; `sample_input` comes from the proposal payload, `null` for other rows), "Test" (only for model, skill and MCP rows; `settings_agent_test_row`) and "show" (`navigateTo` with the setting's concrete key; skill files navigate with `skills[<slug>]`, which `navigateTo` resolves without a catalog entry by switching to Skills and setting `selectedItem`, which `SectionSkills` keeps as the requested slug until the skills store contains it, then selects; prompt files switch to Prompt Base; a secret effect has no "show"). A stale row shows "changed since proposed"; a `stale_rows` result of Apply marks those rows stale in the held proposal; "Refresh" (`settings_agent_refresh`) is offered and Apply stays disabled until none is stale. Apply sends one boolean per operation index and the typed values (below); `stale_rows` or `error` (dependency errors name the dependent rows; a selected `set_env_value` row without a value names its variable) show on the card and nothing is written. Success shows "Applied" and "Undo"; the entry id is in `proposalOutcome` of the store. "Undo" (`undoProposal` of the store) undoes the whole card with `undoChange(entryId, false)`, the outcomes as in `ChangeHistory`: `conflicts` shows `UndoConflicts` with "Undo anyway" (`force: true`), `skipped` shows "Secret backup is on <host>", an error shows on the card, `undone` shows "Undone", a final state: no Apply, no Undo, checkboxes and value inputs disabled. To apply again the user asks the agent for a new card (the backend drops the proposal at Apply). Reject closes the card.

Value input: a row with `env_value` (`set_env_value`) shows an input under the label, a password input with the reveal toggle (`Eye`/`EyeOff`) when `env_value.secret`, a plain text input otherwise; when `env_value.set` the current value is loaded once on mount with `get_secret_value` (a failed load shows the error and leaves the input empty). There is no Save button: the card's Apply writes. The typed values live only in `ProposalCard`'s `$state` record (variable name to value); the store, events, logs and `ui_state` never hold one, and `{#key proposal_id}` drops them with the card. They leave the card only as the `values` argument of `agent.applyProposal(selected, values)` (the selected `set_env_value` rows with a non-empty value) and `agent.testRow(row, values)` (all non-empty values), which pass them on to `settings_agent_apply` and `settings_agent_test_row` and keep nothing. The effect line of such a row shows the variable name only.

Waits and new destination: a row with `waits_for` shows "waits for a value of <names>" in place of its test result, and no "Test" button, while any of those names has no value in the card. A change of a value re-tests, after 500 ms without input or at once on blur, each row whose `waits_for` has that name, whose waited names all have a value and whose `new_destination` is `null`; a value loaded by the prefill re-tests nothing. A row with `new_destination` shows that text in `--warning` until the card is applied; it is tested only by a click on its "Test" button, and a successful test leaves the warning in place.

### Secret field

`SecretField.svelte` (props `name`, `purpose`, `exists`, `masked` (default `true`), `readonly` (default `false`), optional `onclose(outcome)`; the type `SecretOutcome` is exported from its module script): an existing value (`exists`) is loaded on mount with `get_secret_value` for masked and unmasked fields; masked shows it as a password field with a reveal toggle (`Eye`/`EyeOff`, visibility only), `masked: false` as a plain text input without the toggle (for variables that are not secrets, e.g. a path). Save (`save_secret`, writes at once as its own change, independent of any card) and Cancel; Save stays disabled while the value is empty or equal to the loaded one, so opening and saving without an edit writes nothing. A failed load shows the error and leaves the field empty with the "Enter a new value" placeholder. `onclose` gets `saved` or `cancelled`; the value lives only in the component state, never in a store, log or event. The Environment section's Secrets card, `MissingEnvList` and `EnvVarPicker` use this editable mode; the agent does not.

Read-only mode (`readonly`, used for `settings_secret`): `AgentMessages` renders one for each name in `secretViews` of an assistant message, below that message's content. The component loads the value with `get_secret_value` on mount and shows it as bullets with the reveal toggle (`Eye`/`EyeOff`); no input, no Save, no Cancel, `onclose` is not called. A load that returns none or an empty value shows "not set", a failed load shows its error. The store appends the name of the event `settings-agent-secret-view { name }` to `secretViews` of the last assistant message (`emptyReply` starts it empty), so the view keeps its place in the conversation; `reset` clears it with the messages. The agent does not see the value and the field cannot change it: a change goes through a `set_env_value` row of a card.

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

The model API key (`ModelEditor`, `CreateModelDialog`) is an `EnvVarPicker`: a `SelectDropdown` over the listed variables (`getEnvVars()`, refreshed on `settings-changed`) plus "New…"; there is no input that accepts a literal key (the agent rejects literals too). The stored value is `${NAME}`; a picked name emits `${NAME}` and the editor saves at once. Three states come from `classifyReference` (`$lib/utils/envVarOptions`, tested in `tests/envVarOptions.test.mjs`):

- listed: shown as `NAME`, or `NAME (missing)` when no value is set on this machine;
- unlisted reference: `NAME (not listed)` with an "Add to list" button (`add_env_var`, secret on, empty description);
- stored literal: `Literal key` with "Move to a variable": the name/description form, then an empty `SecretField` (the literal is not shown or copied; type the key again); the reference replaces the literal only after the value is saved.

"New…" order: `add_env_var` (inline name/description form, secret on), `SecretField` for the value, then the model write; cancelling the value step still sets `${NAME}`, shown as missing. A rejected operation shows its message under the picker.

`parseEnvRef()` lives in `$lib/utils/envVarOptions` (don't reimplement the regex).

### Surface references

Where a model is used shows as "Used by" tags (`ModelUseTags.svelte`) in three places:

1. `ModelList` rows: tags under the model id and chips, none for an unused model. The row is a `div` holding a selecting `<button>` (name, model id, chips) and the tags, since a button may not contain buttons; hover and active styles sit on the whole row.
2. `ModelEditor` header: a "Used by" line followed by the tags; with no uses, "nothing — " and a link "assign in Assignments ›" (`onShowAssignments`).
3. Delete warning (`DangerZone` `warning` snippet, only with uses): "It is used by <list>." with the same texts as the tags, comma-separated, same order. The warning keeps the `.warn` colour — destructive intent justifies the warning palette.

Tag kinds and texts (`modelUseLabel` in `$lib/constants/surfaces`, shared by tags and warning):

- surface, stored on the surface: solid tag `SURFACE_LABELS[surface]`;
- surface inheriting the Chat model (empty model of Quick actions, Title generation, Settings agent): dashed tag `<label> (same as Chat)`;
- skill pin: tag `skill: <name>`.

Click: a surface tag calls `onShowUse(surface)`; `SectionModels` selects Assignments, sets the local `highlightedSurface` for 2000 ms (a new click restarts the timer), scrolls `#assignment-<surface>` into view and `ModelAssignments` draws the `SettingField` highlight style on that row. The highlight is local state of `SectionModels` because `navigateTo` selects only model ids. A skill tag calls `navigateTo("skills[<name>]")`. A tag click calls `stopPropagation()`, so it never selects the row.

Single source: `store.usesByModel` (`Map<modelId, ModelUse[]>`) and `store.getUsesForModel(id)` (`ModelUse[]`), both from `modelUses` in `$lib/utils/modelUses.ts`: stored and inherited-from-Chat surface uses in the order chat, quick_actions, title_generation, settings_agent, speech_to_text, then skill pins. Never recompute the mapping ad hoc; list, editor header and delete warning must read from the store so they stay in lockstep.

`--warning` is reserved for issue states only ("issue / attention required"); a binding to a surface is not one. Filled circles are reserved for transient state (the save indicator in `SaveStatusIndicator`); persistent structural facts get text.

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

List + editor sections (`SectionModels` with `ModelList`, `SectionSkills` with `SkillList`): the list has no background of its own, list and editor share the section background and are divided only by the list's `border-right: 1px solid var(--border-faint)`, so in the centered column both read as one column. A darker list background stood as a separate strip in the middle of a wide window, with the lighter gutters on both sides. The selected row is `--accent-bg-soft`, hover `--surface-overlay-faint`; both are translucent, so they keep their contrast on the section background. `SectionMcpServers` uses bordered cards on the section background instead of a list pane.

Content column: every section sits in one column of the same width, `--column-width` (1040 px), centered in `.content`, so the column's left edge does not move between sections and on a wide window the free width is split evenly left and right. Inside the column each section keeps its own form widths, left-aligned; never center a section by its own width. `SettingsContent` defines `--column-gutter` (`max(0px, (100% - var(--column-width)) / 2)`, a percentage, so it resolves against the containing block where it is used and is correct only on an element whose containing block spans the whole `.content` width). Applied as `.head` `margin-inline` (the header divider spans the column, not the window) and added to `.body` `padding-inline` in `SettingsSection`, and in `SectionModels` as the model list's `margin-left` and the editor pane's `padding-right`. The scroll containers (`.body`, the models editor pane) keep the full width, so their scrollbar stays at the right edge of the content area; only the Skills editor pane, a scroller inside `.body`, has it at the column's right edge. With the agent panel docked beside the content, `.content` is the width left by the sidebar and the panel: the gutter is 0 while that is at most `--column-width`, and the column is centered in it when wider; with the panel overlaid the column stays centered under it.

`SectionPromptBase` uses a tab strip in the section header — only one `PromptEditor` is mounted at a time and fills the available height. This replaces the old "stacked card-soup" layout where every prompt rendered a full-bordered card stacked vertically. Tabs are uppercase-tracked labels, active tab uses a 2px bottom border in `--accent` (mirrors the sidebar's left-border marker, single-accent rule).

`SectionPromptBase` additionally renders a "Preferred name" identity card above the tabs — a single-field `TextInput` capped at 60 chars (live counter on the right edge, turns `--danger` if `maxlength` is somehow exceeded). It autosaves through `useSaveTracker` + `updateSetting("preferred_name", …)`. The counter and the section's own `SaveStatusIndicator` both belong to this card; the per-editor save dots in the tabs below remain independent.

Editor body width is capped (760px for prose-style prompts, 960px for environment with its placeholders popover; hardcoded in the section styles) to keep long prompts readable; don't let the textarea fill arbitrary window sizes.

Save flow goes through `$lib/services/prompts.ts` → backend `save_prompt` Tauri command → `ConfigService::write_prompt` → `PromptStore` (atomic tempfile + rename). Backend emits `prompt-changed` Tauri event after each save.

### Chat and Speech sections

`SectionChat` shows the default chat model, reasoning effort (as the model editor's effort select: "None" first, all levels when the model does not advertise effort, a saved value the model does not accept shown as "not accepted by this model"; plus "Default" = `null`), the chat's MCP tools (`SurfaceToolsPicker`), the title generation model and the settings agent model (the empty option of both is "Same as Chat (<chat model name>)" from `sameAsChatLabel` in `$lib/utils/modelUses.ts`, = fall back to the chat model), its reasoning effort (options gated by the capabilities of the effective model, as for chat; an empty value falls back to the chat's reasoning effort) and its MCP tools (`SurfaceToolsPicker`, "Settings agent tools"), the title generation prompt (`PromptEditor` kind `title_generation`), and `WebviewProvidersEditor`.

`SectionSpeech` shows STT model, language (empty = `null`), `no_verbatim`, the STT prompt (`PromptEditor` kind `speech_to_text`), keyterms (`SttKeytermsEditor`) and the `speech_to_text_toggle` shortcut, then the recording and failure-handling cards.

`ShortcutField` (prop `action`) lists every shortcut of `action` from all keymap groups of the current OS (`macos` when `isMac`, else `linux`) with `actionShortcuts` from `$lib/utils/keymapShortcuts.ts` (sorted by the stored string, shown through `formatShortcut`: `ctrl+f2` → `Ctrl+F2`), each as a chip; "Not set" when there are none. × on a chip removes that one with `removeShortcut` (the stored string); "+ Add" enters capture: the next keydown with at least one modifier and one key becomes the string `cmd|super`, `ctrl`, `alt`, `shift` (in that order, `cmd` for Meta on macOS, `super` elsewhere) plus the key (`a`, `1`, `f1`…`f24`, `space`, `tab`, `enter`, `backspace`, `delete`, `up`, `down`, `left`, `right`, `home`, `end`, `page_up`, `page_down`), joined by `+`, and is saved with `addShortcut`; Escape or blur cancels. The other shortcuts of the action stay bound. The capture handler calls `stopPropagation()`. The field sits in `SettingField key="keymaps.<action>"`, so a collision error from the backend shows at the field; write failures are logged with `logError`.

`WebviewProvidersEditor` edits the whole `webview_providers` list (name and URL rows, add, remove) and saves it with `updateSetting` — text debounced, remove immediate. Existing ids are kept; a new row gets its id on first save: the name lowercased with characters outside `[a-z0-9_-]` replaced by `-`, suffixed `-2`, `-3`… when taken. Rows without a name are not saved. The list is seeded from the store on mount and not re-synced from later external changes.

`SurfaceToolsPicker` (props `surface`: `"chat" | "settings_agent"` and `enabled`: the surface's list) renders the checkboxes of qualified `server.tool` names from `list_mcp_tools` (empty state "No MCP tools available.", shown below the built-in checkbox when there is one) and, on `surface="settings_agent"`, the "Web search (built-in)" checkbox (value `web_search`) above them, shown only when the effective agent model is OpenAI (or unset provider) in Responses mode (or unset `api_mode`), per `agentModelHasBuiltinSearch` in `$lib/utils/agentModel.ts`; a stored `web_search` stays in the list while the box is hidden, and the chat surface has no built-in checkbox. It writes the whole next list with `updateSurfaceEnabledTools(surface, next)`; a failed write is logged with `logError`.

`SectionContextMenu` shows the quick actions model (the empty option is "Same as Chat (<chat model name>)") and reasoning effort (options follow the effective model, `quickActionsModelId`; `effortOptions` from `$lib/utils/effortOptions.ts`, shared with `SectionChat`), `menu_section_order`, `recent_apps_count` (`NumberInput`, min 0, saved after `autosave_debounce_ms`) and `ShortcutField` for `open_context_menu`, `execute_active_prompt`, `set_context_value`, `append_context_value`, `clear_context`. The order list shows the ids present in the setting, named (`chat` Chat; `skills`, `prompts`, `SkillProvider` Skills; `models` Models; `settings` Settings; `speech`, `SpeechMenuProvider` Speech; `lastInteraction`, `LastInteractionMenuProvider` Last interaction; `context`, `ContextMenuProvider` Context; an unknown id as is). HTML5 drag-and-drop with a grip handle moves an item; a drop saves the whole list with every id exactly as stored, so legacy aliases are never rewritten to canonical ids and no section is added or removed.

### MCP Servers section

`SectionMcpServers` edits the whole `mcp_servers` map: list on the left with a state chip per server (`running`, `error`, `starts after restart` for a configured server the registry did not try, `removed after restart` for one it ran that is no longer configured; states from `listMcpServers()` in `$lib/services/mcp.ts`, read once on mount), editor on the right (a header with the server name and its `SaveStatusIndicator`, as in `ModelEditor`; then name, command, args one per line, env rows, timeout, tool timeout rows). A failed server shows its start error in full, a running one its tools. Rows are a local draft seeded on mount; every edit saves the whole map through `updateSetting("mcp_servers", map)` (text debounced via `useSaveTracker`, delete immediate after an inline two-step confirm). Duplicate names are not saved and show an error at the name. Backend errors show at `mcp_servers.<name>` (name) and in `SettingField` for the catalog keys; `SettingField` gets no `itemId` there because its `itemId` match expects `[id]` keys and these are dotted. Changes apply after restart.

Env rows have column headers ("Name in server", "Value"), shown when the server has at least one env row, and one row shape: name `TextInput`, a value cell, the remove button. The value cell holds a `TextInput` (placeholder "value") for any value that is not a single `${NAME}` token, including an empty one or a mixed one such as `Bearer ${TOKEN}`; a single `${NAME}` shows as a `Chip` without `onclick` (a `<span>`, so the buttons inside are not nested in a button) with the name and, once the first env load finished, its state from `classifyReference` and the list: `NAME · set`, `NAME · missing` (listed without a value) or `NAME · not listed`. Inside the chip: "Add to list" only for `not listed` (`add_env_var`, secret on, then reload; a rejection shows under the row without its `row N: ` prefix and changes nothing) and an × button ("Clear value") that stores an empty value and brings back the text field. Typing a complete `${NAME}` into the text field turns it into the chip. The stored value stays the `${NAME}` string. The env list (`getEnvVars()`) loads on mount, on `onSettingsChanged` and on window `focus`; a failed load logs a warning and keeps the old list. Each value cell has an `InsertVariableButton` after the field or chip: with a text field it inserts `${NAME}` at the caret, with a chip there is no input, its `field` getter returns null and a pick replaces the value with `${NAME}`. The getter finds the row's input through a per-row wrapper (`valueCells[pair.key]`, `querySelector("input")`), as `command` does. Env names stay free text. The name input keeps a fixed 38 % (`.pair.env > .text-input`, the header's first column matches it): `TextInput` is `width: 100%`, so without that share it takes the whole row and the value cell collapses to 0 px, its field disappears and the following remove button is drawn over it.

`InsertVariableButton.svelte` (lucide `Braces`, next to `command` and `args`) opens a `FloatingPanel` with `MenuList` + `MenuItem` rows listing `getEnvVars()` (read on every open) in two labelled groups, Secrets first and Variables second, each sorted by name, a group without names omitted (`groupByKind` in `$lib/utils/envVarOptions`; the label is a non-clickable `div` styled in the component, not a `MenuItem`); a name without a value shows `missing` in the row's `shortcut` slot. Picking a name, when `field()` returns a field, inserts `${NAME}` at the caret or over the selection of its field through `insertAt()` (`$lib/utils/envVarOptions`), writes the new text into the field and to the bound row value through `oninsert`, then refocuses the field with the caret after the token. When `field()` returns null (an env row showing a chip), a pick only calls `oninsert` with `${NAME}`, which replaces the value. The caret and selection survive the blur caused by the click, so the button needs no focus tracking; in `args` the caret is on the line being edited. Escape closes the list and stops propagation. `field` is a getter because `TextInput` spreads extra props onto its `<input>` and exposes no element ref; `command` finds its input through the wrapper (`querySelector("input")`).

### Search and navigation

`SettingsSearch` (top of the sidebar) matches the query words as prefixes of words of an entry's catalog label or description (`searchCatalog` in `$lib/utils/settingsSearch.ts`; label matches first, max 20). Arrow keys move, Enter or click navigates and clears the query, Escape clears; its keydown handler stops propagation.

`navigateTo(key)` of `getSettingsNavigation()` (`$lib/stores/settingsNavigation.svelte.ts`) is the single entry point for jumps, also for the settings agent's links. `key` is a catalog or concrete path (`models[<id>].api_key`, `mcp_servers.<name>.env`). It switches `activeSection` (owned by that store), sets `selectedItem` when the concrete path names a list item (`SectionModels` and `SectionMcpServers` select it), scrolls `setting-<catalog path>` into view and sets `highlighted` for 2000 ms; `SettingField` draws an accent outline while its catalog path matches. A key `section:<id>` is a section link: a known id switches `activeSection`, scrolls every scrolled element inside `[data-settings-content]` (the `main` of `SettingsContent`) back to the top and sets `highlightedSection` for 2000 ms; `SettingsSection` draws the accent outline on its title `h2` and `SettingsSidebar` on the section's item while `highlightedSection` is the active section (`SectionModels` has no title, so only its sidebar item); an unknown id logs `warn` and changes nothing.

### Change history

`ChangeHistory` is a card at the bottom of `SectionHistory`: the settings agent's change-log entries of all hosts, newest first (`listChangeLog` in `$lib/services/changeLog.ts`). Each entry shows a title (`entryTitle` in `$lib/utils/changeLog.ts`: row labels for a card, `Secret <name>`, `Undo of <time of the undone entry>`), local time, host, kind, the request, and a state badge `applied` / `undone`. Expanding it shows each row's effects as before → after (file effects as a line diff) and test results; a secret effect (`set_env_value`) and a secret entry show only the variable name (`targetLabel` in `$lib/utils/changeLog.ts`, also used by `ProposalRow` and `conflictLines`, whose line for a secret conflict reads `<NAME>: changed since this entry`). Undo calls `undoChange(id, false)`: `conflicts` shows `UndoConflicts.svelte` (shared with `ProposalCard`), an inline list of targets with current vs expected value and "Undo anyway" (`force: true`); `skipped` shows "Secret backup is on <host>"; `undone` reloads the list. The list reloads on mount, on `change-log-changed` and on window focus.

### Window registration reminders

The settings dialog is a separate Tauri window. When adding new windows nearby (e.g., a sub-dialog), register the HTML entry in `vite.config.ts` `rollupOptions.input` AND the label in `src-tauri/capabilities/default.json`: `"windows"` for windows, `"webviews"` for child webviews (`conversation-dialog`, `shell-toolbar`). See [project root CLAUDE.md](../../../../../CLAUDE.md). Prefer not to spawn child dialogs from settings — inline confirms cover all current cases.
