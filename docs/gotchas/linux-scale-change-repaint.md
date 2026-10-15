# Stale window drawing after a scale change on Linux

## Rule

Keep `services/scale_repaint.rs` registered (Linux only, `lib.rs`). It repairs every window after its GTK `scale-factor` changes; do not remove it or add per-window workarounds.

## Symptom

On GNOME Wayland a window moved between monitors of different scale (e.g. 150 % and 100 %) keeps stale drawing: a white unpainted area and smeared content of other windows. Each further move makes it worse. A manual resize repairs it.

## Trigger

GTK3 integer buffer scale switching 2 ↔ 1. Moves between monitors of equal scale do not glitch.

## Ruled out

- `WEBKIT_DISABLE_DMABUF_RENDERER=1`: glitch stays.
- `WEBKIT_DISABLE_COMPOSITING_MODE=1`: glitch stays.
- The chat window's build: the settings window (decorated, one webview) glitches too.

The failing layer (WebKitGTK, GTK3, Mutter or the NVIDIA driver) is not isolated. Seen on GNOME Shell 50.1, NVIDIA RTX 3050 Ti, WebKitGTK 2.52.6.

## Tried and rejected

- `queue_resize` + `queue_draw` without a size change: no repair.
- 1 px width nudge on the next idle: repairs only intermittently.

## Why a GTK hook

tao 0.34 emits no `WindowEvent::ScaleFactorChanged` on Linux (it only stores the value on GTK `scale-factor` notify), so window event handlers cannot see the change. The repair connects to GTK `scale-factor` notify itself.

## The repair

Plugin `scale-repaint` (`services/scale_repaint.rs`), connected through `on_window_ready`, so every window kind gets it once:

- 300 ms debounce after the last `scale-factor` notify; a newer notify cancels the pending nudge (generation counter).
- Then a 1 px width nudge, restored 100 ms later.
- The restore only runs when the current width is still `width + 1` and keeps the current height; an app resize in between is kept.
- Notifies during the own nudge and for 100 ms after the restore are ignored, so a window edge on the monitor border cannot start a loop.
- Maximized and fullscreen windows are skipped.
- Timers hold weak references to the window.

The nudge emits `Resized` and `Moved`; dialog geometry is saved only on close or by explicit command, so a wrong size is not persisted.

Watch it with `RUST_LOG=app_lib::services::scale_repaint=debug`.
