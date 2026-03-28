# Context Menu

Borderless popup window that renders menu items fetched from the Rust `MenuCoordinator` backend.

## Architecture

The context menu runs in a **separate Tauri window** (`context-menu` label) with its own HTML entry point (`context-menu.html` → `ContextMenuApp.svelte`). This window is borderless, transparent, always-on-top, and hidden by default.

### Files

| File | Role |
|------|------|
| `ContextMenu.svelte` | Root UI — renders sections, handles keyboard/mouse interaction |
| `$lib/stores/contextMenu.svelte.ts` | Reactive state — items, selection, open/close lifecycle |
| `context-menu.html` (project root) | HTML entry point for the context-menu window |
| `src/ContextMenuApp.svelte` | Minimal root that mounts `ContextMenu` |
| `src/context-menu-main.ts` | JS entry point for the context-menu window |

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
| Escape | Close menu |
| 1-9 | Quick-select item by position (multi-digit debounced at 300ms) |

## Rich item types

The item rendering loop checks `item.item_type` and delegates to specialized components:

### ContextSection (`ContextSection.svelte`)

Renders when `item.item_type === "context"`. Extracts `ContextItem[]` from `item.data.items`.

| Element | Description |
|---------|-------------|
| Section header | Collapsible, shows "Context" label + item count badge |
| Text chip | Truncated text preview (first 50 chars) |
| Image chip | Shows format label (PNG/JPEG/etc.) derived from media type |
| Copy button | Copies concatenated text context to clipboard (hidden when no text items) |
| Clear button | Calls `clearContext()` to remove all context items |

The context store (`$lib/stores/context.svelte.ts`) is initialized in `ContextMenuApp.svelte` so the section updates reactively via the `"context-changed"` Tauri event.

### Prompt execution

When a `prompt` item is clicked, the context menu store intercepts it (instead of calling the generic `execute_menu_item` backend command) and delegates to the execution store's `startExecution(promptId)`. The menu closes immediately, and the execution runs asynchronously.

During execution, prompt items are disabled. The store applies this by overlaying `enabled: false` on prompt items when `isExecuting()` is true (see `applyExecutionState` in `contextMenu.svelte.ts`). On `execution-completed`, the store refreshes items so they re-enable on next open.

The execution store is initialized in `ContextMenuApp.svelte` alongside the context store.

### Other types

Speech, settings, and other item types are rendered as simple label + icon buttons and use the generic `execute_menu_item` backend command. Future tasks will add specialized renderers as needed.
