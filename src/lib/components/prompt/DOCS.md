# Prompt Components

Conversation dialog UI: input area, message bubbles, sidebar tabs, palette, tool calls.

Each entrypoint is a thin orchestrator. IPC, listeners, timers, and pagination live in drivers (`*.svelte.ts`) or services. Repeated visual fragments live in dumb primitives in `components/`.

## Layers

- **Orchestrators** (top-level files in this dir) — wire props to drivers and primitives. May call services.
- **Dumb primitives** (`components/`) — pure presentation, own `<style>`, take props + callbacks. **No** `invoke()`, `services/`, or `stores/` imports.
- **Drivers** (`drivers/*.svelte.ts`) — runes-based logic (state machines, IPC, listeners, timers). **No** JSX or CSS.
- **Services** (`$lib/services/`) — IPC adapters and external-system wrappers.

## Files

| File | Role |
|------|------|
| `AssistantBubble.svelte` | Orchestrator — markdown, tool-call grouping, edit segments via `useEditSegments` |
| `AttachMenu.svelte` | Plus-button dropdown (context + tool toggles) |
| `ChatPalette.svelte` | Cmd+K palette wrapping `useHistorySearch` |
| `ContextSection.svelte` | Inline context bar (text/image chips + clear/edit) |
| `ConversationArea.svelte` | Scroll container + auto-scroll via `useAutoScroll` |
| `InputArea.svelte` | Orchestrator — composer, attachments, tools, model select |
| `JsonNode.svelte` | Recursive JSON renderer |
| `SearchResultsRenderer.svelte` | Web search XML envelope renderer |
| `TabSidebar.svelte` | Orchestrator — list + rename + delete via mutex driver |
| `ToolCallGroup.svelte` | Header (`Running tools` shimmer / `Used N tools` summary) + items |
| `ToolCallItem.svelte` | Single tool call — header, status, expand, retry |
| `ToolChip.svelte` | Active tool dismissable chip |
| `ToolResultRenderer.svelte` | Tool output (envelope / json / text/markdown) |
| `UserBubble.svelte` | Orchestrator — read/edit user message + attachments |
| `XmlNodeRenderer.svelte` | Recursive XML renderer |
| `sidebarItems.ts` | Pure helpers building `SidebarItem[]` from tabs + history |
| `$lib/utils/historySearchSnippet.ts` | `displayName`, `snippetFor`, `formatTimestamp` for `ChatPalette` |

### `components/`

| File | Used by |
|------|---------|
| `AttachmentRow.svelte` | `InputArea`, `UserBubble` |
| `BranchNav.svelte` | `AssistantBubble` |
| `BubbleActionsFooter.svelte` | `AssistantBubble`, `UserBubble` |
| `BubbleEditField.svelte` | `AssistantBubble`, `UserBubble` |
| `ErrorBanner.svelte` | `AssistantBubble` |
| `InlineRenameInput.svelte` | `TabSidebar` |
| `ProcessingIndicator.svelte` | `AssistantBubble`, `ToolCallGroup` |
| `SidebarItemRow.svelte` | `TabSidebar` |
| `SidebarMoreMenu.svelte` | `TabSidebar` |
| `TabSidebarHeader.svelte` | `TabSidebar` |
| `ToolCallApprovalActions.svelte` | `ToolCallItem` (pending state) |
| `ToolCallDetails.svelte` | `ToolCallItem` (expanded view) |
| `ToolCallHeader.svelte` | `ToolCallItem` |
| `ToolCallReadOnlyChip.svelte` | `AssistantBubble` (edit mode) |

### `drivers/`

| File | What it owns |
|------|--------------|
| `useAutoScroll.svelte.ts` | scroll-to-bottom, manual-scroll-up gate, `loadMore` with prevHeight correction |
| `useConversationsList.svelte.ts` | `getConversations` pagination + `history-changed` listener |
| `useEditSegments.svelte.ts` | parse / build / rebuild bubble edit segments + textareaRefs auto-resize |
| `useElapsedTimer.svelte.ts` | live elapsed-seconds via `setInterval` while active, frozen on completion |
| `useInputSync.svelte.ts` | local↔store sync for text/images/textAttachments + `lastDomText` race protection + `SkillEditable` resync on tab change |
| `useMcpTools.svelte.ts` | `list_mcp_tools` + `mcp-ready` listener; exposes `webSearchQualifiedId` |
| `useSidebarMutex.svelte.ts` | mutually exclusive menu / rename / confirm-delete state |
| `useTextAttachmentBridge.svelte.ts` | `text-attachment-updated` listener — patches local attachments by index |

### Services used

| Service | Used by |
|---------|---------|
| `$lib/services/clipboardPaste.ts` | `InputArea` (Mac `Shift+Cmd+V` raw-text/image branch) |
| `$lib/services/fileSave.ts` | `AssistantBubble`, `ToolResultRenderer` (Mermaid SVG export) |
| `$lib/services/windowPreviews.ts` | `InputArea`, `UserBubble` (open text/image preview windows) |
| `$lib/services/history.ts` | `TabSidebar` (`updateHistoryEntryTitle`, `deleteHistoryEntry`) |

## Layer contracts

- `components/*` and `$lib/components/ui/*` **must not** import `services/`, `stores/`, or `invoke()`. Take props, emit callbacks.
- `drivers/*.svelte.ts` are pure runes — **no JSX, no CSS**. May call `invoke()`, attach window listeners, run timers, use `$effect`.
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
