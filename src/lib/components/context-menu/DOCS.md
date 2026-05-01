# Context Menu

Borderless popup window that renders menu items fetched from the Rust `MenuCoordinator` backend.

## Architecture

The context menu runs in a **separate Tauri window** (`context-menu` label) with its own HTML entry point (`context-menu.html` → `ContextMenuApp.svelte`). This window is borderless, transparent, always-on-top, and hidden by default.

`ContextMenu.svelte` is split into three layers:

- **Dumb UI primitives** in `components/` — pure presentation, own `<style>`, no IPC/stores/services.
- **Logic drivers** in `drivers/` (`*.svelte.ts`) — runes-based state machines for window/keyboard/blur/panel concerns. No JSX, no CSS.
- **Data adapters** in `$lib/services` and `$lib/stores` — fetching, caching, and derived data.

`ContextMenu.svelte` is the orchestrator: instantiates drivers, wires adapters, lays out the template.

### Files

| File | Role |
|------|------|
| `ContextMenu.svelte` | Orchestrator — wires drivers/adapters, renders sections via dumb children |
| `MenuShell.svelte` | Outer chrome (border, padding, shell layout) |
| `ContextSection.svelte` | Renders `context` items (chips, edit mode, action buttons) |
| `LastInteractionSection.svelte` | Renders `last_interaction` items (input/output/transcription chips + history) |
| `itemExtractors.ts` | `groupBySection`, `extractContextItems`, `extractLastInteractionData`, last-interaction types |
| `components/MenuEmptyState.svelte` | "No items available" fallback |
| `components/MenuSeparator.svelte` | 1px section divider |
| `components/MenuItemRow.svelte` | Generic row (icon + prompt-number + label + executing/recording state) |
| `components/ChatRow.svelte` | Chat row (icon + label + recording toggle) |
| `components/SettingsToggleRow.svelte` | Chevron + "Settings" toggle row |
| `components/FooterHint.svelte` | Bottom hint (`⇧ voice input · right-click for actions`) |
| `components/SkillActionMenu.svelte` | Floating panel content for skill RMB action menu (open in dialog / mic / metadata) |
| `components/SettingsPanel.svelte` | Floating panel content for Settings (quick action model + STT model selectors) |
| `drivers/useFloatingPanelMutex.svelte.ts` | Mutually exclusive 3-panel state (settings / action menu / chat providers) + anchors |
| `drivers/useMenuKeyboard.svelte.ts` | `keydown`/`keyup` handler, shifted digits, `shiftHeld` flag |
| `drivers/useMenuBlurClose.svelte.ts` | Window blur listener, suppressed-blur recheck timer, blur-grace passthrough |
| `drivers/useMenuPositioning.svelte.ts` | Resize/position window at cursor anchored to skills section, hover-enable gate |
| `$lib/services/skillMetadata.svelte.ts` | Singleton cache for `get_skill` metadata + `buildSkillMetaEntries` |
| `$lib/stores/webviewProviders.svelte.ts` | Webview providers list + `onSettingsChanged` listener + Promptheus prepend |
| `$lib/stores/modelsMenuData.svelte.ts` | Derived `modelsData`/`modelNames`/`quickActionModel` + capability prefetch + setters |
| `$lib/stores/contextMenu.svelte.ts` | Reactive state — items, selection, open/close lifecycle, IPC listeners |
| `$lib/stores/useContextMenu.svelte.ts` | Getter wrapper around `contextMenu.svelte.ts` |
| `context-menu.html` (project root) | HTML entry point for the context-menu window |
| `src/ContextMenuApp.svelte` | Minimal root that mounts `ContextMenu` |
| `src/context-menu-main.ts` | JS entry point for the context-menu window |

### Layer contracts

- `components/*` and other UI primitives **must not** import `services/`, `stores/`, or `invoke()`. They take props and emit callbacks.
- `drivers/*.svelte.ts` are pure runes — no JSX, no CSS. They may call `invoke()` (positioning calls `show_context_menu_panel` / `focus_context_menu`) or attach window listeners (blur).
- Mutex `open*` methods atomically close the other two panels, removing the previous bug-prone `closePanels(); settingsOpen = true;` pattern.
- `skillMetadata` cache is a module-level singleton (`$state`) so it survives menu close/reopen — fewer redundant `get_skill` IPC calls.

### Window configuration

Defined in `src-tauri/tauri.conf.json` under `app.windows` with label `context-menu`. Key properties: `decorations: false`, `transparent: true`, `alwaysOnTop: true`, `visible: false`.

## Interaction flow

1. Trigger (tray "Show Menu" or future global hotkey) calls `show_context_menu_window` backend command.
2. Backend positions the window at cursor, emits `show-context-menu` event to the window, then shows and focuses it.
3. Store's `init()` listener catches the event → calls `openMenu()`.
4. `openMenu()` invokes `get_context_menu_items` command and populates state (window is already visible).
5. User interacts via keyboard or mouse → executes item via `execute_menu_item` command.
6. After execution (or Escape/blur), `closeMenu()` hides the window.

## Keyboard shortcuts

| Key | Action |
|-----|--------|
| Arrow Up/Down | Move selection highlight |
| Enter | Execute selected item (Shift+Enter for alternative execution) |
| Escape | Close menu (closes any open floating panel first) |
| 1-9 | Quick-select item by position (multi-digit debounced at 300ms) |
| Shift+1-9 | Same as above with alternative execution flag |

All keyboard handling lives in `drivers/useMenuKeyboard.svelte.ts`.

## Floating panels

Three mutually exclusive panels (`drivers/useFloatingPanelMutex.svelte.ts`):

| Panel | Trigger | Contents |
|-------|---------|----------|
| Settings | LMB on Settings toggle row | `SettingsPanel` — quick action model + STT model |
| Skill action menu | RMB on a skill row | `SkillActionMenu` — open in dialog / run with transcription / metadata |
| Chat providers | RMB on Chat row | `ProviderMenuList` — Promptheus + webview providers |

Closing the Settings or Skill action menu triggers `menu.resumeClose()` to release the suppression any inner control (e.g. ModelSelector) may have set.

## Rich item types

The item rendering loop checks `item.item_type` and delegates to specialized components:

### ContextSection (`ContextSection.svelte`)

Renders when `item.item_type === "context"`. Extracts `ContextItem[]` from `item.data.items`.

| Element | Description |
|---------|-------------|
| Section header | Collapsible, shows "Context" label + item count badge |
| Text chip | Truncated text preview (first 50 chars) |
| Image chip | Shows format label (PNG/JPEG/etc.) derived from media type |
| Replace button | Replaces context with clipboard content via `setContextFromClipboard()` |
| Append button | Appends clipboard content to context via `appendContextFromClipboard()` |
| Edit button | Toggles inline edit mode with `ContextEditor` component |
| Copy button | Copies concatenated text context to clipboard (disabled when no text items) |
| Clear button | Calls `clearContext()` to remove all context items (disabled when empty) |

Action buttons are always visible (not gated by empty state) since Replace/Append/Edit are useful even with no context.

**Collapse behavior**: collapsed by default when empty, auto-expands when items exist. The `$effect` on `items.length` drives this.

**Edit mode**: when active, replaces chips with `ContextEditor` (textarea + image chips). Save persists via `clearContext()` + `setContext()` + `setContextImage()`. The `onEditingChange` callback notifies `ContextMenu.svelte` to suppress blur-close and keyboard handling.

The context store (`$lib/stores/context.svelte.ts`) is initialized in `ContextMenuApp.svelte` so the section updates reactively via the `"context-changed"` Tauri event.

### Skill execution

When a `skill` item is clicked, the context menu store intercepts it (instead of calling the generic `execute_menu_item` backend command) and delegates to the execution store's `startExecution(skillId)`. The menu closes immediately, and the execution runs asynchronously.

During execution, skill items are disabled. The store applies this by overlaying `enabled: false` on skill items when `isExecuting()` is true (see `applyExecutionState` in `contextMenu.svelte.ts`). On `execution-completed`, the store refreshes items so they re-enable on next open.

The execution store is initialized in `ContextMenuApp.svelte` alongside the context store.

### LastInteractionSection (`LastInteractionSection.svelte`)

Renders when `item.item_type === "last_interaction"`. The backend injects last interaction data into `item.data` in the `get_context_menu_items` command (from `HistoryService`).

| Element | Description |
|---------|-------------|
| Section header | "Last interaction" label + History button |
| Input chip | Last text entry's input content (copy on click) |
| Output chip | Last text entry's output content (copy on click) |
| Transcription chip | Last speech entry's output content (copy on click) |

Chips are disabled (grayed out) when no content is available. Copy uses `copyHistoryContent` from the history service which also triggers a clipboard notification. The store refreshes on `"history-changed"` events so chips update after each execution.

History button is a placeholder — the history dialog window is not yet implemented.

### Other types

Speech, settings, and other item types are rendered as `MenuItemRow` (label + icon + optional prompt number) and use the generic `execute_menu_item` backend command. Future tasks will add specialized renderers as needed.
