# AI webview idle freeze on Linux — three protection layers

## Rule

Keep all three layers below working when changing the AI webview lifecycle. None of them is
sufficient alone; each catches the problem at a different stage.

## Symptom

After tens of minutes of idle (often with the monitor asleep), the app stops responding while
a ChatGPT/Claude provider webview is open: the context-menu hotkey still fires (the
`tauri-plugin-global-shortcut` handler runs on its own thread), but the action never
completes, clicks do nothing, and the window manager offers "Wait / Force Quit".

## Why

Provider webviews are children of the host window (`add_child`, Tauri `unstable` feature) and
share the process's GTK/GLib main loop with every other window. When a provider's WebKit
content process stops answering, any main-thread call that waits synchronously stalls with it:
`cursor_position()`, `available_monitors()`, IPC to another webview in the same window,
`x11_get_server_time` before `present_with_time`, `set_focus()`. The suspected triggers are a
lost compositor context after monitor sleep, a stuck long-lived websocket/SSE stream, and heap
growth in a page kept open for hours. Moving providers to separate windows or separate
`WebKitWebContext`s does not help: they still share the main loop.

## Layer 1 — WebKit memory pressure (Linux)

`services/webkit_pressure.rs → install`, called first in `setup/init.rs → run`. Sets
`MemoryPressureSettings` per content process: limit 2048 MB, conservative 0.50, strict 0.75,
kill 0.95, poll 30 s. WebKit frees memory in long-lived pages instead of growing until OOM.

- Must run before the first webview exists; `WebsiteDataManager::set_memory_pressure_settings`
  has no effect afterwards. Keep it above `setup::windows::create`.
- Needs `webkit2gtk` with feature `v2_34` in `Cargo.toml` (also exposes the Layer 2 APIs).

## Layer 2 — responsiveness watchdog (Linux)

`services/ai_webview/lifecycle.rs → install_media_permissions` connects
`is-web-process-responsive` notify. On `responsive=false` it waits
`UNRESPONSIVE_GRACE_SECONDS` (20 s, `cold_suspend.rs`) via `glib::timeout_add_seconds_local_once`;
if still unresponsive it calls `terminate_web_process()` + `reload()` and shows a Warning toast
("<provider> was unresponsive"). The same function logs `web-process-terminated` (observation
only, no reload).

- Works only while the main loop still runs: the signal and the GLib timer are both dispatched
  on it. A hang that blocks the main loop first is out of its reach; Layer 3 covers that case.
- Any handler that reloads on `web-process-terminated` must skip deliberate terminations,
  or terminate → reload loops.
- No macOS equivalent: WKWebView has no public responsiveness API.

## Layer 3 — cold suspend after long idle (all platforms)

`setup/background.rs → spawn_ai_webview_cold_suspend` runs
`services/ai_webview/cold_suspend.rs → run_cold_suspend_pass` on the main thread every
`COLD_SUSPEND_POLL_INTERVAL` (60 s). Candidates (`AiWebviewState::cold_suspend_candidates`)
are provider webviews (`ai-webview-*` or `*::ai-webview-*`) not active in any host, not already
suspended, idle longer than `COLD_SUSPEND_IDLE_THRESHOLD` (90 min). Each is parked: its current
URL is saved with `mark_suspended`, then it navigates to `about:blank`. No toast.

`resume_if_suspended` navigates back to the saved URL (the exact conversation, not the provider
home). It is called from `provider_swap.rs → hosted_swap_to_provider` and
`lifecycle.rs → open_or_focus`. Cookies and storage survive, so the user sees a short reload.

- Every new path that shows a hidden provider webview must call `resume_if_suspended`,
  otherwise the user lands on a blank page.
- Uses public `navigate` rather than `terminate_web_process`, so it works on macOS too.
- Suspending immediately on hide was rejected: it kills instant switching between providers.
  Lowering the threshold trades that switching speed for earlier protection.

## Diagnostics

- `setup/background.rs → spawn_heartbeat` logs `heartbeat dispatch #N` from tokio and
  `heartbeat tick #N delay=…` from the main thread every 5 s. Dispatches without ticks = main
  loop blocked; the gap in numbers × 5 s = freeze length.
- `commands/menu.rs → show_context_menu_window` / `focus_context_menu` and
  `services/dialog.rs → present_on_main` log per-step elapsed times; the last step without a
  successor is the call that stalled.
- Layer logs: `responsiveness changed`, `web process unresponsive >20s, restarting`,
  `web process terminated`, `cold-suspend: webview=… parked`.

## When to load this file

- Changing `services/ai_webview/` lifecycle, provider swap or cold suspend, or
  `services/webkit_pressure.rs`.
- Investigating an AI webview window freezing after idle on Linux.

## Related

- [tauri-command-threading.md](tauri-command-threading.md) — blocking work on the GTK main thread.
- [linux-gtk-focus.md](linux-gtk-focus.md) — `present_with_time` / `x11_get_server_time`.
