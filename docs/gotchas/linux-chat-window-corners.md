# Rounded corners and border of the chat window on Linux

## Rule

On GNOME Wayland the chat window's corners are cut and its border is drawn by the pages, over a transparent host. Do not move that work to the GTK side: a cairo clip or stroke on the box holding the child webviews has no effect on WebKitGTK children.

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

## Traps

- A `body` background with `html` transparent moves to the canvas (CSS Backgrounds 3 §2.11.2), and the canvas ignores `border-radius` and `clip-path` of `body`: square corners filled with the dim or colour. Keep `html` painted by `linear-gradient(transparent, transparent)` or paint on a pseudo-element.
- `overflow: hidden` with a radius on `html` clips nothing (root overflow goes to the viewport) and does not clip `position: fixed` descendants; use `clip-path` on a full-viewport `body`.
- The provider script runs before `<head>` exists on first load: it waits for `DOMContentLoaded` when `document.documentElement` is missing.
- Whether a provider page's own `html`/`body` background, painted on the canvas, stays inside the injected root `clip-path` depends on the page: the Claude tab was OK live (2026-11-11), ChatGPT not confirmed.
- The state is one process-wide flag (one chat host window).

Live results per criterion: `src-tauri/src/services/DOCS.md`, "Chat window corners (GNOME Wayland)". Watch the state with `RUST_LOG=app_lib::services::chat_corners=debug`.
