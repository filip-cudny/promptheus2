# Linux/GTK window focus — `present_with_time` with a real X11 timestamp

## Rule

On Linux, never rely on Tauri's `set_focus()` alone to bring a window forward. Use `gdk_window().present_with_time(timestamp)` with a **real X11 timestamp** obtained from `gdkx11::functions::x11_get_server_time()`. Reference implementation: `focus_context_menu` in `src-tauri/src/commands/menu.rs`.

## Why

Tauri's `Window::set_focus()` internally calls GTK's `gtk_window_present_with_time(GDK_CURRENT_TIME)`. `GDK_CURRENT_TIME` is the constant `0`. Modern X11 window managers (GNOME/Mutter, KDE/KWin) treat a focus request with timestamp `0` as a **focus-steal attempt** by an app that wasn't user-initiated, and silently demote it to `_NET_WM_STATE_DEMANDS_ATTENTION` (taskbar flash) instead of actually focusing.

Symptom: the window appears but doesn't take keyboard focus, or the taskbar entry pulses while the user has to click manually.

To pass the WM's "is this user-initiated?" check, the timestamp must come from a **recent X server event**. `x11_get_server_time()` round-trips to the X server and returns a fresh server-side timestamp — the WM accepts it as legitimate.

## Pattern

```rust
#[cfg(target_os = "linux")]
{
    use gdkx11::functions::x11_get_server_time;
    use gtk::prelude::GtkWindowExt;

    if let Some(gtk_window) = window.gtk_window().ok() {
        if let Some(gdk_window) = gtk_window.window() {
            let timestamp = unsafe { x11_get_server_time(&gdk_window) };
            gtk_window.present_with_time(timestamp);
        }
    }
}
```

## When to load this file

- Adding a new window that needs to grab focus (palette, dialog, notification recall).
- Investigating a "window appears but no keyboard focus" / "taskbar flashes instead of activating" bug on Linux.
- Touching any code that calls `set_focus()`, `show()`, or `unminimize()` on Linux windows.

## Related

- macOS doesn't need this — `set_focus()` works, but on macOS context menus need a non-activating panel pattern (see commits `eef369f`, `03d3a58`).
- For window registration (capabilities, Vite inputs), see the workflow rules in `CLAUDE.md`.
