# Conversation Dialog

Renders the chat window mounted by `windows/conversation-dialog/App.svelte`: composer, message bubbles, history sidebar, palette, tool calls.

Each entrypoint is a thin orchestrator. IPC, listeners, timers, and pagination live in drivers (`*.svelte.ts`) or services. Repeated visual fragments live in dumb primitives in `components/`. Tool-call rendering is its own subsystem under `tool-call/`.

## Layers

- **Orchestrators** (top-level files in this dir) — wire props to drivers and primitives. May call services.
- **Dumb primitives** (`components/`) — pure presentation, own `<style>`, take props + callbacks. **No** `invoke()`, `services/`, or `stores/` imports.
- **Drivers** (`drivers/*.svelte.ts`) — runes-based logic (state machines, IPC, listeners, timers). **No** JSX or CSS.
- **Tool-call subsystem** (`tool-call/`) — self-contained renderers for tool invocation UI. Only `AssistantBubble` reaches into it (via `ToolCallGroup`), apart from `components/ToolCallDetails`; the rest is internal.
- **Services** (`$lib/services/`) — IPC adapters and external-system wrappers.

## Files

Files whose role does not follow from the name:

- `AssistantBubble.svelte`, `UserBubble.svelte`, `InputArea.svelte`, `TabSidebar.svelte` — orchestrators; the two bubbles use `ActionIconButton` for the footer buttons (`active` on the Edit button, `variant="danger"` on Delete); `AssistantBubble` edits its segments through `drivers/useEditSegments`, `UserBubble` through `BubbleEditField` and `SkillEditable` with its own `editMode`/`editText`.
- `ContextSection.svelte` — inline context bar (text/image chips + clear/edit).
- `sidebarItems.ts` — pure helpers building `SidebarItem[]` from history.
- `$lib/utils/historySearchSnippet.ts` — `displayName`, `snippetFor`, `formatTimestamp` for `ChatPalette`.
- `tool-call/` — self-contained tool-invocation renderer; external entrypoint `ToolCallGroup` (used by `AssistantBubble`).
- `components/ToolCall*.svelte` — pieces of `tool-call/ToolCallItem`; `ToolCallDetails` imports `tool-call/ToolResultRenderer`.
- `drivers/useConversationDialogIpc.svelte.ts` — the dialog's IPC listeners.
- `drivers/useInputSync.svelte.ts` — local↔store sync for text/images/text attachments (see "Input sync"); `drivers/useSidebarMutex.svelte.ts` — see "Sidebar mutex".

## Layer contracts

- `components/*` and `$lib/components/shared/ui/*` **must not** import `services/`, `stores/`, or `invoke()`. Take props, emit callbacks.
- `drivers/*.svelte.ts` are pure runes — **no JSX, no CSS**. May call `invoke()`, attach window listeners, run timers, use `$effect`.
- `tool-call/*` is internal beyond `ToolCallGroup`; outside the dir, import only `tool-call/ToolCallGroup.svelte` (exception: `components/ToolCallDetails.svelte` imports `tool-call/ToolResultRenderer.svelte`).
- Orchestrators wire it all up: instantiate drivers, pass adapters as callbacks, render primitives.
- Mutex `open*` methods atomically clear other states (no `closeMenu(); editingId = …` two-step that desyncs).

## Flows

### Bubble edit (assistant)

1. User clicks Pencil → `toggleEditMode()` → `useEditSegments.enter(displayContent)` builds segmented state and resizes textareas next frame.
2. User types in any segment → `edit.onSegmentInput(idx, e)` updates that segment + auto-resizes its textarea.
3. User hits Ctrl+Enter / Save → `edit.rebuild()` collapses segments back into `{{tool_call:id}}` markers. `onContentChange(rebuilt)` propagates to store.
4. `edit.exit()` clears segments + textareaRefs.

### Input sync

`useInputSync` runs four `$effect`s:

1-3. local → store on every local change (gated by `activeTabId === syncedTabId` to skip writes during the tab handover).

4. store → local on every store change. On tab change OR when text changed externally, calls `skillEditable.setTextAndHighlight` + `resetUndoStack` + (next frame) `focus + restoreCursor`. `lastDomText` tracks last DOM-driven write to detect external mutations.

This avoids feedback loops while preserving the cursor on user typing and resetting it on history-restore / tab-switch.

### Sidebar mutex

`useSidebarMutex` holds three states (`menuOpenId`, `editingId`, `confirmDelete`). Each `open*` / `start*` calls `closeAll()` first, so opening any one atomically closes the other two. Outside-click and Escape route through `closeAll()` / `clear*` to keep the model consistent.

### Global context loading

`getGlobalContext()` (`stores/conversation.svelte.ts`) reads `get_context_items` and `get_labelled_context` and returns `{ text, images }`: `text` is the labelled `<pasted-text name="Text #N">` blocks (empty without text items), `images` carry the item `number`. The numbers go into the chat prompt (`[Context Image #N]`), token count, execution and history (`image_index = number - 1`). `ContextSection.saveContext` runs `unwrapContextBlocks` on the field (only `<pasted-text name="Text #N">` blocks; text outside them is kept; the `\n\n` between consecutive blocks becomes `\n`), saves it as one text item and reloads the tab from `getGlobalContext()`. Two callers:

1. `ContextSection` starts its local text/images from the active tab's stored context (so mounting never overwrites context the store already loaded) and on mount refills them from it.
2. `store.loadGlobalContextIntoActiveTab()` writes it into the active tab; `handleVoiceInput` awaits it before `sendMessage()` when `autoSend`, then shows the context section collapsed, so the auto-sent message carries the context (a window just created via `applyInitParams`, a window already open via the `voice-input` event).

### Voice-input source

`DialogInitParams` and the `voice-input` payload carry `skill_id`, `skill_model` and a nullable `invocation_source`. `handleVoiceInput` builds `/<skill_id> <text>` (plain text for an empty id), calls `store.setActiveTabInvocationSource(source ?? "alternative_execute")` and, for a non-empty `skill_model`, `store.setActiveTabModelId(skill_model)` (the active tab only, not `updateModelId`). Sources: the pill sends `pill`, the context menu's "Run in dialog" sends `alternative_execute`.

Both also carry a nullable `copy_token`, set only by pill stop + chat. With a token, `handleVoiceInput` calls `copy_chat_answer_if_unchanged(token, text)` once after the auto-sent first answer, only when the send succeeded, its tab is still open and its assistant node is not `cancelled`; `text` is the send's `result`. The backend command decides on the clipboard fingerprint. On `true`, `store.markAutoCopied(nodeId)` sets `autoCopiedNodeId` for 1200 ms: `ConversationArea` passes `autoCopied` to that `AssistantBubble`, which shows its footer and the Copy button's check. No toast. A window that reconnects (accepted limit in "One window") or is closed before the answer ends copies nothing. Follow-ups, regenerate and "Send and copy" never carry a token.

## One window

`conversation-dialog` is the only chat window; no command takes a host label. `open_conversation_dialog` writes `PENDING` only when it creates the window; when the window exists (visible or hidden) it goes through the surface step and delivers by event, so input is never re-sent.

Accepted limit: when the one global live-stream slot is set, a window created at that moment reconnects (`tryReconnect`) and ignores the result of `get_dialog_init_params` (it still invokes it), so an auto-send request ("Run in dialog", the pill's "Stop and chat") opens the dialog without sending.

## Tabs

Native chat tabs and provider tabs share the strip of `shell-toolbar`; the conversation store stays the owner of native tabs.

- Native sync: `App.svelte` reports a snapshot (`report_native_tabs`: `tab_id`, title, `streaming`, `listed`) after every change of the store's tabs, after `shell:active-changed` and once after mount. `listed` = the tab is not clean, or it is the active tab while the native view is visible; clean unlisted tabs stay out of the strip.
- Rust drives the store through events: `chat:activate-tab` (`{ tab_id }`) switches the store's tab, `chat:close-tab` (`{ tab_id, next_tab_id }`) removes it without picking a successor; `new-conversation` with `reason: "tab"` runs `addTab()`.
- `TabSidebar` lists history only; clicking an entry restores it with `store.restoreFromHistory`, deleting an entry also closes its tab.
- Closing a tab (strip, shortcut) never deletes its history entry; only the drawer's delete does.
- Shortcuts (`$lib/shortcuts.ts`, handled in the chat webview, the strip and the provider webviews): Ctrl+Tab / Ctrl+Shift+Tab next / previous (wraps), Cmd/Ctrl+1–8 the n-th tab, Cmd/Ctrl+9 the last, Cmd/Ctrl+T a new tab of the active tab's provider (native when the active tab is native; nothing when no tab exists), Cmd/Ctrl+W close the active tab.
