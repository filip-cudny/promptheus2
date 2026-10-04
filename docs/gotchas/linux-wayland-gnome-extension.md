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
| `GetFocusedWmClass` | `() → s` | `wm_class` of the focused window |
| `ShortcutActivated` | signal `s action` | an accelerator fired |
| `Ready` | signal | the extension is enabled and serving |

## Pattern

Lifecycle:
- The extension emits `Ready()` in `enable()` and after unlock (extensions are disabled on the lock screen).
- Rust calls `SetShortcuts` again after every `Ready`, so shortcuts survive unlock and an app started before the extension was enabled.
- The extension watches the bus name of the `SetShortcuts` caller and ungrabs its accelerators when the name vanishes (app exit or crash), so the keys return to the focused app.
- Shortcuts are grabbed for the normal and overview action modes only: they do not fire on the lock screen.
- Accelerators come from `get_active_bindings` converted by `to_gtk_accelerator` in `services/hotkeys.rs`. An accelerator GNOME already uses is returned by `SetShortcuts` and logged at `warn`; the other shortcuts keep working.

Window identification:
- Windows are found by title. The context menu title is `CONTEXT_MENU_TITLE` (`commands/menu.rs`, used in `setup/windows.rs`); change it in that one constant.
- Any process on the session bus can call `PlaceWindow` and `SetShortcuts`; there is no caller check.

Context menu placement:
- GTK3 on Wayland creates the surface at `show()`, so the window does not exist for Mutter before it.
- `place_context_menu_through_shell` puts the `PlaceWindow` call on the bus before `win.show()` and then awaits the reply. The extension waits up to 1000 ms for the window, keeps its compositor actor hidden (`opacity = 0`) until the first frame, then moves, activates and shows it.
- GTK `set_opacity` cannot do this: it writes an X11 atom and is X11-only (see [linux-webkit-opacity.md](linux-webkit-opacity.md)).
- `GetPointer` supplies the cursor and work area; the frontend positions inside the work area and calls `place_context_menu` (see `src/lib/components/features/context-menu/DOCS.md`).

Install:
- `.deb`: `bundle.linux.deb.files` in `src-tauri/tauri.conf.json` installs `metadata.json` and `extension.js` into `/usr/share/gnome-shell/extensions/promptheus@promptheus.desktop/`.
- Development: `linux/gnome-shell-extension/install-dev.sh` symlinks the directory into `~/.local/share/gnome-shell/extensions/`.
- Then re-login and `gnome-extensions enable promptheus@promptheus.desktop`. The app never installs or enables the extension itself.
- `metadata.json` `shell-version` lists exact GNOME majors (`["50"]`); add the new major on each GNOME release.
- Not verified yet: whether `gnome-extensions disable`/`enable` reloads a changed `extension.js` or a re-login is needed.

Debugging:
- `gdbus introspect --session --dest org.gnome.Shell --object-path /com/promptheus/Shell` — 4 methods, 2 signals when the extension is enabled.
- `gdbus monitor --session --dest org.gnome.Shell` — watch `Ready` and `ShortcutActivated`.
- `journalctl --user /usr/bin/gnome-shell` — extension errors.

## When to load this file

- Touching global shortcuts, window positioning or window focus on Linux.
- Changing `linux/gnome-shell-extension/`, `services/gnome_shell/`, `setup/shortcuts.rs`, `place_context_menu` or `useMenuPositioning.svelte.ts`.
- Investigating "shortcuts do nothing on Ubuntu / GNOME", "menu opens at the wrong place or without focus on Wayland", or "recent apps are empty on Wayland".

## Related

- [linux-gtk-focus.md](linux-gtk-focus.md) — the X11 focus path.
- [linux-webkit-opacity.md](linux-webkit-opacity.md) — X11-only window opacity.
- `src-tauri/src/services/DOCS.md` — `gnome_shell` and hotkeys sections.
