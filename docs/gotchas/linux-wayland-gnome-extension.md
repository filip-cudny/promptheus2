# GNOME Wayland — shortcuts, positioning and focus through the GNOME Shell extension

## Rule

On GNOME Wayland, Promptheus runs as a native Wayland client. Wayland has no protocol for global shortcuts, window positioning or window activation by a client, and `tauri-plugin-global-shortcut` only grabs keys through XWayland (they arrive while an X11 window has focus). Everything that needs the compositor goes through the GNOME Shell extension `promptheus@promptheus.desktop` (`linux/gnome-shell-extension/`), reached over the session D-Bus by `src-tauri/src/services/gnome_shell/`. Do not add `setPosition`, `set_focus()`, `present_with_time` or plugin shortcut code for this path.

## Why

- The extension runs inside the compositor: it can grab accelerators, read the pointer and the monitor work area, move and activate a window, and read the focused window's `wm_class`.
- `org.gnome.Shell.Eval` is blocked since GNOME 41, and `xdotool` does not see Wayland windows.
- Backend selection (`services/gnome_shell::is_gnome_wayland`): `XDG_SESSION_TYPE=wayland` and `XDG_CURRENT_DESKTOP` contains `GNOME` → extension backend, always. Anything else (macOS, X11, other desktops) → `tauri-plugin-global-shortcut` and the old placement path. Without the enabled extension on GNOME Wayland there are no shortcuts (`warn` log) and the context menu opens only from the tray, at the old, wrong position (`shell_placement: false`).

## D-Bus contract

Interface `com.promptheus.Shell`, object `/com/promptheus/Shell`, served by `org.gnome.Shell`:

| Member | Signature | Meaning |
|---|---|---|
| `SetShortcuts` | `a{sas} → as` | action → GTK accelerators; returns the accelerators GNOME refused (already taken) |
| `GetPointer` | `() → (iiiiii)` | pointer x, y, then work area x, y, width, height of the monitor under the pointer |
| `PlaceWindow` | `(s title, i x, i y, b activate) → b` | move the window with that title, optionally activate it |
| `PlaceWindowAnchored` | `(s title, i right, i bottom, b activate) → b` | keep the frame's bottom-right corner at `(right, bottom)` until the window is unmanaged, optionally activate it |
| `GetFocusedWmClass` | `() → s` | `wm_class` of the focused window |
| `ShowToast` | `(s level, s title, s message, b monochromatic) → ()` | draw a toast as a shell actor; `level` is `success`/`error`/`info`/`warning`, empty `message` = no description |
| `ShowRecordingWidget` | `(i x, i y, b has_position) → ()` | show the recording widget at `(x, y)` logical stage pixels when `has_position`, else at the default position |
| `UpdateRecordingWidget` | `(s state, d level, u elapsed_ms) → ()` | `state` is `recording`/`paused`/`processing`/`done`; `level` 0..1 |
| `HideRecordingWidget` | `() → ()` | remove the widget |
| `ShortcutActivated` | signal `s action` | an accelerator fired |
| `RecordingWidgetAction` | signal `s action` | widget button pressed: `pause`, `resume`, `stop` or `cancel` |
| `RecordingWidgetMoved` | signal `(i x, i y)` | drag released after the pointer moved more than 4 px, logical stage pixels |
| `Ready` | signal | the extension is enabled and serving |

## Pattern

Lifecycle:
- The extension emits `Ready()` in `enable()` and after unlock (extensions are disabled on the lock screen).
- Rust calls `SetShortcuts` again after every `Ready`, so shortcuts survive unlock and an app started before the extension was enabled.
- The extension watches the bus name of the `SetShortcuts` caller and ungrabs its accelerators when the name vanishes (app exit or crash), so the keys return to the focused app.
- Shortcuts are grabbed for the normal and overview action modes only: they do not fire on the lock screen.
- Accelerators come from `get_active_bindings` converted by `to_gtk_accelerator` in `services/hotkeys.rs`. An accelerator GNOME already uses is returned by `SetShortcuts` and logged at `warn`; the other shortcuts keep working.

Window identification:
- Windows are found by title. The context menu title is `CONTEXT_MENU_TITLE` (`commands/menu.rs`), the notification title is `NOTIFICATION_TITLE` (`commands/notification.rs`); both are set in `setup/windows.rs`. Change a title only in its constant.
- Any process on the session bus can call `PlaceWindow` and `SetShortcuts`; there is no caller check.

Context menu placement:
- GTK3 on Wayland creates the surface at `show()`, so the window does not exist for Mutter before it.
- `gnome_shell::place_window(title, x, y, activate, Some(win))` puts the `PlaceWindow` call on the bus before `win.show()` and then awaits the reply. The extension waits up to 1000 ms for the window, keeps its compositor actor hidden (`opacity = 0`) until the first frame, then moves it, activates it when `activate` is set, and shows it. With `None` it only sends `PlaceWindow` to an already visible window. `place_context_menu_through_shell` calls it with `activate = first`.
- A second `PlaceWindow` for the same title while the first still waits cancels the first (it returns `false`, logged at `warn`) and is handled on its own.
- GTK `set_opacity` cannot do this: it writes an X11 atom and is X11-only (see [linux-webkit-opacity.md](linux-webkit-opacity.md)).
- `GetPointer` supplies the cursor and work area; the frontend positions inside the work area and calls `place_context_menu` (see `src/lib/components/features/context-menu/DOCS.md`).

Notification toasts:
- On GNOME Wayland `commands/notification.rs::show_notification` calls `ShowToast` (async, proxy method `show_toast`) and does not touch the webview queue. The extension draws the toast as St actors in `Main.layoutManager.uiGroup`: not windows, never focused, non-reactive (no hover pause).
- Durations are set in the extension (`TOAST_DURATIONS_MS`: success 2000, error 4000, info 2000, warning 3000 ms). The stack sits bottom-right in the work area of the monitor current when the stack went from empty to non-empty, newest at the bottom.
- Each toast actor eases in and out with the values of GNOME Shell's window map/destroy animation for normal windows (`windowManager.js` `_mapWindow`/`_destroyWindow`): enter from pivot (0.5, 1.0), scale 0.01 × 0.05, opacity 0 to scale 1 and opacity 204 in 150 ms `EASE_OUT_EXPO`; exit at pivot (0.5, 0.5) to scale 0.8 and opacity 0 in 150 ms `EASE_OUT_QUAD`, after which the actor is destroyed and the remaining toasts move to their new place without animation.
- A failed `ShowToast` call logs `warn` and shows that payload through the webview window path below. That path is also the only one on X11 and macOS.
- Why actors: Mutter ignores the GTK3 `accept_focus` hint (`focusable(false)`) for Wayland clients and focuses the mapped notification window anyway, even with `activate=false`; focus returns only when `hide()` unmanages it.

Recording widget:
- Drawn by the extension as a reactive St actor added with `Main.layoutManager.addTopChrome` (unlike the non-reactive toasts, it receives clicks above windows): no modal (`Main.pushModal`), no `global.stage.grab`, so the client keeps keyboard focus. Buttons are `St.Button` children; dragging the pill body is handled in the shell (a `captured-event` handler on the stage for the duration of the drag).
- Drag: `button-press-event` on the pill installs a stage `captured-event` handler; it moves the pill only after the pointer moved more than `WIDGET_DRAG_THRESHOLD_PX` (4 px) from the press point, and only then stops the release and emits `RecordingWidgetMoved`. A press whose event actor (`global.stage.get_event_actor(event)`) is an `St.Button` or inside one never starts a drag.
- Pitfall: a stage `captured-event` handler sees events before every actor. When it returned `Clutter.EVENT_STOP` for the `BUTTON_RELEASE` of a press that started on a child `St.Button` (the press bubbles to the pill), the button never got the release and never emitted `clicked`; each click only logged `recording widget dragged to X,Y`. Any stage capture handler must propagate events it does not own.
- `ShowRecordingWidget` position is validated by the shell against the current monitors; outside every work area (or `has_position = false`) it uses the default, bottom center 24 px above the work area bottom of the pointer's monitor.
- Hidden by `HideRecordingWidget`, when the bus name of the `SetShortcuts` caller vanishes, and in `disable()`.
- Rust side: `services/speech/widget.rs` (see `src-tauri/src/services/speech/DOCS.md`); position stored under `recording_widget.shell_position`. A failed `ShowRecordingWidget` makes the recording use toasts.

Notification placement (webview window path):
- `commands/notification.rs` reads `GetPointer` and sends `PlaceWindowAnchored(NOTIFICATION_TITLE, work_right, work_bottom, activate=false)` before the first `show()`, with the bottom-right corner of the work area under the pointer. The window is `focusable(false)`, which Mutter does not honor for Wayland clients (see above).
- The extension sets the frame to x = `right` − frame width, y = `bottom` − frame height from `get_frame_rect()`. It applies this at placement (on `first-frame` when it had to wait for the window), once more on the next idle, and on every `size-changed` of the `MetaWindow`. It moves only when the computed x/y differ from the current frame position, so its own move does not re-trigger it.
- The anchor is stored per title; a new `PlaceWindowAnchored` for the same title replaces it. It is released on the window's `unmanaged` signal: GTK3 on Wayland destroys the surface on `hide()`, so each show cycle is a new `MetaWindow` and a new `PlaceWindowAnchored`.
- `.always_on_top(true)` in `setup/windows.rs` maps to GTK `keep_above`, which has no Wayland protocol under GTK3 (`xdg-shell` has no stacking request); on Wayland it does nothing.
- The extension calls `make_above()` in `_anchorWindow` on every anchor, so the window stacks above app windows; every window placed with `PlaceWindowAnchored` is kept above. The window is `focusable(false)`; without `make_above` Mutter stacks it as a normal never-focused window under the focused app, and an occluded surface gets no frame callbacks (resize answered 20–30 s late).
- `update_notification_window` on the extension path only calls `set_size`; the extension follows the resize. The X11/macOS path keeps `set_position` then `set_size`. `anchor_and_show` calls `set_size` (380 × `FIRST_SHOW_HEIGHT`) before showing, because after `hide()` the GTK window keeps its size from the previous cycle.
- Why the anchor lives in the extension: the GTK client commits a new size asynchronously. A top-left computed in Rust from the requested height reached Mutter before or after the real resize, so the frame kept the old height at the new origin or the new height at the old origin, and the toast stack (anchored to the window bottom by Sonner `position="bottom-right"`) ended below the work area. Resizing before `PlaceWindow` did not fix it. Only the compositor knows the real frame size.
- The notification drain closes every toast still live `DURATIONS[level] + 1500` ms after it showed (own timer per toast plus a sweep at the start of every `drainPending`) and logs a `warn` with the source (`timer`/`sweep`) and page state, because sonner timers have been seen to stall on GNOME Wayland (cause not yet known).
- A toast paused under the pointer is closed the same way and also logs the `warn`.
- The drain logs at `debug` the toast show/close lines (close with path `autoClose`/`dismiss`/`watchdog`), toaster mouse/pointer events, window focus/blur/keydown (`altKey` and whether the code is `KeyT` only, never the key) and `visibilitychange`; kept in the product for diagnosing a future stall.
- `update_notification_window` logs at `debug` the toast count, requested logical height and either the extension anchor (bottom-right) or the computed origin.
- `GetPointer` coordinates are logical stage pixels, so the shell anchor uses scale 1.0; the X11/macOS path keeps physical pixels and `set_position`.
- If the extension call fails (extension disabled), the show path logs at `warn` and falls back to `cursor_position` + `find_monitor_at` + `set_position`; Mutter ignores the position and places the toast itself.
- `services/monitor.rs::find_monitor_at` falls back to the monitor under the cursor, then `primary_monitor()`, then the first of `available_monitors()`. On Wayland `cursor_position()` is not global and GDK reports no primary monitor; without the last fallback the notification never showed (`no monitor found`).

Install:
- `.deb`: `bundle.linux.deb.files` in `src-tauri/tauri.conf.json` installs `metadata.json`, `extension.js` and `icons/` into `/usr/share/gnome-shell/extensions/promptheus@promptheus.desktop/`.
- Development: `linux/gnome-shell-extension/install-dev.sh` symlinks the directory into `~/.local/share/gnome-shell/extensions/`.
- Then re-login and `gnome-extensions enable promptheus@promptheus.desktop`. The app never installs or enables the extension itself.
- `metadata.json` `shell-version` lists exact GNOME majors (`["50"]`); add the new major on each GNOME release.
- A changed `extension.js` needs a re-login: GNOME Shell caches loaded ES modules, so `gnome-extensions disable`/`enable` re-runs the old code.

Debugging:
- `gdbus introspect --session --dest org.gnome.Shell --object-path /com/promptheus/Shell` — 9 methods, 4 signals when the extension is enabled.
- `gdbus monitor --session --dest org.gnome.Shell` — watch `Ready` and `ShortcutActivated`.
- `journalctl --user /usr/bin/gnome-shell` — extension errors.
- `journalctl --user -f -o cat /usr/bin/gnome-shell | grep Promptheus` — anchored placement lines: `Promptheus: make_above "<title>"` on every anchor, then `Promptheus: anchor "<title>" right=R bottom=B moved=… frame=X,Y WxH`, then `re-anchor after placement` and `re-anchor on size-changed` with the same fields, and `anchor released "<title>"` on hide. A correct anchor has `X + W = R` and `Y + H = B`.

## When to load this file

- Touching global shortcuts, window positioning or window focus on Linux.
- Changing `linux/gnome-shell-extension/`, `services/gnome_shell/`, `setup/shortcuts.rs`, `place_context_menu`, `useMenuPositioning.svelte.ts`, `commands/notification.rs` or `services/monitor.rs`.
- Investigating "shortcuts do nothing on Ubuntu / GNOME", "menu opens at the wrong place or without focus on Wayland", "notifications never appear on Wayland", or "recent apps are empty on Wayland".

## Related

- [linux-gtk-focus.md](linux-gtk-focus.md) — the X11 focus path.
- [linux-webkit-opacity.md](linux-webkit-opacity.md) — X11-only window opacity.
- `src-tauri/src/services/DOCS.md` — `gnome_shell` and hotkeys sections.
