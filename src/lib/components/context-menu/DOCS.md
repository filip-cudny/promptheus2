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

1. Backend emits `show-context-menu` event (from tray "Show Menu" or global hotkey).
2. Store's `init()` listener catches the event → calls `openMenu()`.
3. `openMenu()` invokes `get_context_menu_items` command, populates state, shows the window.
4. User interacts via keyboard or mouse → executes item via `execute_menu_item` command.
5. After execution (or Escape/blur), `closeMenu()` hides the window.

## Keyboard shortcuts

| Key | Action |
|-----|--------|
| Arrow Up/Down | Move selection highlight |
| Enter | Execute selected item (Shift+Enter for alternative execution) |
| Escape | Close menu |
| 1-9 | Quick-select item by position (multi-digit debounced at 300ms) |

## Extending with rich item types

The current shell renders all items as simple label + icon buttons. Future feature tasks (context, prompts, speech, settings) will add specialized renderers by checking `item.item_type` and `item.data` to render widgets (chips, toggles, etc.) for their specific section types.
