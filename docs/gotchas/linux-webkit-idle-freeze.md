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
`set_focus()`. The suspected triggers are a
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
`UNRESPONSIVE_GRACE_SECONDS` (20 s, `unload.rs`) via `glib::timeout_add_seconds_local_once`;
if still unresponsive it calls `terminate_web_process()` + `reload()` and shows a Warning toast
("<provider> was unresponsive"). The same function logs `web-process-terminated` (observation
only, no reload).

- Works only while the main loop still runs: the signal and the GLib timer are both dispatched
  on it. A hang that blocks the main loop first is out of its reach; Layer 3 covers that case.
- Any handler that reloads on `web-process-terminated` must skip deliberate terminations,
  or terminate → reload loops.
- No macOS equivalent: WKWebView has no public responsiveness API.

## Layer 3 — unloading idle provider tabs (all platforms)

Provider tabs are child webviews of the one `conversation-dialog` host (`services/ai_webview/tabs.rs → TabList`). Unloading (`unload.rs → unload_tab`) closes the child webview and keeps the tab with its URL and title, so a hung or bloated content process is gone while the tab stays in the strip. Three triggers:

- Idle pass: `setup/background.rs → spawn_ai_webview_unload` runs `unload.rs → run_unload_pass` on the main thread every `UNLOAD_POLL_INTERVAL` (60 s). Candidates (`TabList::idle_unload_candidates`) are live provider tabs that are not active, have no armed pill message and were last used `UNLOAD_IDLE` (30 min) ago. "Last used" is the later of activation and deactivation.
- Live limit: at most `MAX_LIVE_PROVIDER_TABS` (3) live provider tabs; making another live unloads the least recently used one that is not active and not armed.
- Window hide: `surface.rs → hide_host` unloads every live provider tab.

Activating an unloaded tab (`provider_swap.rs → activate_tab`) creates a new child at the saved URL (the exact conversation, not the provider home) with a new label; cookies and storage survive, so the user sees a 3–6 s reload and loses scroll position and unsent composer text. An armed pill message on an unloaded tab goes to the pill fallback at once.

- Every path that shows a provider tab must go through `activate_tab`, otherwise the user lands on a tab without a webview.
- Unloading by closing the child works on macOS too.
- Unloading immediately on hide was rejected: it kills instant switching between tabs. Lowering `UNLOAD_IDLE` trades that switching speed for earlier protection.

## Diagnostics

- `setup/background.rs → spawn_heartbeat` logs `heartbeat dispatch #N` from tokio and
  `heartbeat tick #N delay=…` from the main thread every 5 s. Dispatches without ticks = main
  loop blocked; the gap in numbers × 5 s = freeze length.
- `commands/menu.rs → show_context_menu_window` / `focus_context_menu` and
  `services/dialog.rs → present_on_main` log per-step elapsed times; the last step without a
  successor is the call that stalled.
- Layer logs: `responsiveness changed`, `web process unresponsive >20s, restarting`,
  `web process terminated`, `unload pass: candidates=…`, `unloaded tab=… label=…`.

## When to load this file

- Changing `services/ai_webview/` lifecycle, tab unloading, or
  `services/webkit_pressure.rs`.
- Investigating an AI webview window freezing after idle on Linux.

## Related

- [tauri-command-threading.md](tauri-command-threading.md) — blocking work on the GTK main thread.
