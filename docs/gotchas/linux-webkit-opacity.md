# Transparent WebKitGTK windows — CSS opacity does not work

## Rule

On a Linux window created with `.transparent(true)`, **no CSS-level opacity technique** produces visible transparency on the static rendering of an inner element. Use **GTK-level window opacity** via `gtk_window().set_opacity(value)` instead. Reference implementation: notification window in `src-tauri/src/commands/notification.rs` (or wherever `set_opacity` is called — search for it).

## Why

WebKitGTK uses two different paint paths:

- **GPU composite path** — engaged briefly during CSS transitions/animations. Honors per-element transparency.
- **CPU paint path** — the default for static frames. Always opaque, regardless of what CSS says.

So a fade-out transition appears to work for a moment, but as soon as the element settles into a static state it pops back to full opacity. None of these workarounds help on a transparent webview window:

- `opacity: 0.5`
- `background: rgba(0, 0, 0, 0.5)`
- `filter: opacity(0.5)`
- `will-change: opacity`
- `@keyframes` that hold a partially transparent value

## What actually works

Set the opacity at the GTK window level. This sets the X11 atom `_NET_WM_WINDOW_OPACITY`, which the compositor honors before WebKitGTK's paint path even matters.

```rust
#[cfg(target_os = "linux")]
{
    use gtk::prelude::GtkWindowExt;
    if let Ok(gtk_window) = window.gtk_window() {
        gtk_window.set_opacity(0.85);
    }
}
```

This is window-wide — you can't make individual elements transparent this way. If you need per-element transparency, the only paths are: don't use a transparent webview window (use a solid background and color the elements directly), or accept that the effect only renders during animation.

## When to load this file

- Designing or styling any window created with `.transparent(true)` on Linux.
- Investigating "elements look fully opaque on Linux but transparent on macOS".
- Adding fade-in/fade-out effects on transparent windows.

## Related

- macOS handles transparency via Cocoa NSVisualEffectView; the same CSS techniques work fine there.
- For background-color setup on transparent windows, see the Tauri `Color` helper used in commit `42bf8f4`.
