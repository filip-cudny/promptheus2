# Rounded corners, border and shadow of the chat window on Linux

## Rule

On GNOME Wayland the chat window's corners are cut and its border is drawn by the pages, over a transparent host. Do not move that work to the GTK side: a cairo clip or stroke on the box holding the child webviews has no effect on WebKitGTK children. The shadow is the exception: it lies outside the webviews, in a transparent margin of the host surface, so GTK CSS on that box draws it.

## Symptom

`conversation-dialog` has no system frame and no shadow (`decorations(false)`); over a window with the same background (Settings) its edge is invisible. After making the host transparent and clipping a 10 px rounded rectangle on the `GtkBox` (`default_vbox()`) that holds the child webviews, the top corners were round (toolbar page CSS) and the bottom corners stayed square.

## Why the GTK side cannot do it

- GDK3 on Wayland applies no visual window shape, only an input region: a corner is cut only by alpha, so the host must be transparent and every layer above it must paint alpha 0 at the corners.
- WebKitGTK children do not paint into the parent's cairo context: the clip set on the box before they draw is not honoured, and a border stroked on the same context after them lies beneath them.
- A decorated window (GTK client-side frame) disables `attach_undecorated_resize_handler`, adds the Adwaita shadow, shifts the content below y=0 and still does not clip the bottom corners.

## Fix pattern

Only on GNOME Wayland (`gnome_shell::is_gnome_wayland()`); other sessions keep the opaque square window.

- Host built `.transparent(true)` with alpha-0 background; toolbar, native chat and provider children get alpha-0 backgrounds (`SHELL_BG` elsewhere). Applies to children created later and re-created after unload (`create_provider_child`).
- Toolbar page: top corners by CSS radius, border on top, left, right.
- Native chat page: `html` and `body` transparent, `clip-path: inset(0 round 0 0 10px 10px)` on `body` (also clips fixed descendants), border by `body::after`.
- Provider pages (Claude, ChatGPT): `provider_corners_script` injects one style with the same clip-path and an `html::after` border; it is an initialization script of the child, re-evaluated on every page load and on each corner state change.
- Palette and backdrop pages: bottom corners and left, right, bottom border from the same state, sent when the palette opens.
- State (`chat_corners::rounded_for_state`): square while maximized, fullscreen or tiled (including `*_TILED` flags), rounded otherwise; sent to the pages as `shell:corners` and read on mount with `get_chat_corners`.

## Shadow (rounded state only)

`chat_corners::apply_shadow` on each state change, on the GTK main thread:

1. `default_vbox()` gets the style class `chat-window-shadow` (own `CssProvider`: `border-radius: 10px; box-shadow: 0 10px 14px rgba(0,0,0,.55), 0 0 0 1px rgba(0,0,0,.35)`); a `GtkBox` draws outset shadows outside its border box.
2. Widget margins on the box, 28 left and right, 18 top, 38 bottom (`ROUNDED_SHADOW_WIDTH`), make room for the shadow inside the surface.
3. `gdk::Window::set_shadow_width` with the same values: GDK Wayland grows the surface by them, keeping the visible size, and sends the visible rect as the xdg window geometry on every commit, so Mutter maximizes, tiles and places `PlaceWindowOnParent` children against the visible window.
4. Square state: no class, margins and shadow width 0.

- Input region: on every `configure-event` (GDK Wayland also sends one after each map) the GtkWindow gets `input_shape_combine_region` of the visible rect grown by `RESIZE_BAND_IN_SHADOW` (10 px), `None` while square; clicks on the rest of the shadow reach the window below.
- Resize: `dialog::edge_at` (unit-tested) takes the box margins: from 10 px outside the visible edge to 5 px × scale inside it. The webview handlers cover the inside, `attach_shadow_resize_handler` on the GtkWindow (connected after tao's, so its cursor wins) the band in the margin.
- Size: tao's `inner_size` is the whole surface, margin included. The window is built at the saved visible size; `save_geometry` and the palette and backdrop size subtract `chat_corners::shadow_width()`.

## Traps

- A `body` background with `html` transparent moves to the canvas (CSS Backgrounds 3 §2.11.2), and the canvas ignores `border-radius` and `clip-path` of `body`: square corners filled with the dim or colour. Keep `html` painted by `linear-gradient(transparent, transparent)` or paint on a pseudo-element.
- `overflow: hidden` with a radius on `html` clips nothing (root overflow goes to the viewport) and does not clip `position: fixed` descendants; use `clip-path` on a full-viewport `body`.
- The provider script runs before `<head>` exists on first load: it waits for `DOMContentLoaded` when `document.documentElement` is missing.
- Whether a provider page's own `html`/`body` background, painted on the canvas, stays inside the injected root `clip-path` depends on the page: the Claude tab was OK live (2026-11-11), ChatGPT not confirmed.
- The state is one process-wide flag (one chat host window).
- A `gtk_window_set_decorated` call on the host (tao `set_decorations`) resets the GDK shadow width to 0: the visible rect is then the whole surface again, margin and shadow included, until the next state change. Call `apply_shadow` after it.
- GTK's CSS blur radius is about the standard deviation of its blur, a browser's twice that: the mockup's `28px` is `14px` in GTK CSS. GTK draws that blur out to 40 px; the margins cut it at 28 px from the shadow box, at about 1 % alpha (upgrade path in the comment on `ROUNDED_SHADOW_WIDTH`).
- GDK Wayland drops the input region with the surface on hide and only re-sends it after `input_shape_combine_region` marks it dirty: set it again after each map, or a reopened window blocks clicks on its whole shadow.
- GDK Wayland `set_shadow_width` reconfigures before it stores the new margins, so the size kept for the first configure of the next map (`unconfigured_width/height`, used when Mutter's first configure is 0x0 and also before the first configure) subtracts the old margins: after a square to rounded change the chat window reopens, and the first show opens, 56 × 56 px larger, and `save_geometry` keeps the growth. `apply_shadow` calls `gdk::Window::resize` to the same surface size (visible size + new margins) right after it, which only recomputes that size with the new margins. Rounded branch only: while maximized, fullscreen or tiled `move_resize` sets `saved_size_changed` and breaks the restore size after unmaximize.
- Window position is not restored on Wayland, so `save_geometry` does not adjust x and y by the margin.

Live results per criterion: `src-tauri/src/services/DOCS.md`, "Chat window corners (GNOME Wayland)". Watch the state with `RUST_LOG=app_lib::services::chat_corners=debug`.
