# Tauri command threading & GTK main thread blocking

## Rule

Any `#[tauri::command]` that performs blocking work (clipboard, filesystem, subprocess, network, X11 calls) **must** be marked `#[tauri::command(async)]` or written as `pub async fn`. A plain sync `pub fn` command is invoked **on the GTK main thread** on Linux — blocking it freezes the entire app: UI rendering, IPC, heartbeat, every other command.

## Why

Tauri 2 routes commands through different executors based on signature:

| Form | Runs on |
|---|---|
| `#[tauri::command] pub fn ...` | **Main thread** (GTK loop on Linux) |
| `#[tauri::command(async)] pub fn ...` | `tauri::async_runtime::spawn_blocking` worker |
| `#[tauri::command] pub async fn ...` | tokio task on the async runtime |

Sync commands run on main thread by design — Tauri needs that thread to safely touch the WebView (GTK is not thread-safe). The cost: any blocking call in a sync command stalls the GUI.

On Linux X11 there's an additional self-deadlock path. `arboard::Clipboard::get_text()` sends a `SelectionRequest` to the clipboard owner and blocks waiting for `SelectionNotify`. If our own WebKitGTK webview owns the selection (because the user just copied something inside the app), that request must be serviced by **our** GTK event loop — which is blocked inside `arboard`. After ~4 seconds arboard times out. Repeated invocations serialize; pressing the shortcut 5–6 times produces 20+ second freezes.

## Symptom in logs

```
[INFO][arboard::platform::linux::x11] Time-out hit while reading the clipboard.
[INFO][app_lib::heartbeat] heartbeat tick #N delay=21.87s
```

The heartbeat tick measures `app_handle.run_on_main_thread(...)` callback latency. Any multi-second delay there means the GTK loop is blocked.

## Fix pattern

```rust
#[tauri::command(async)]
pub fn get_clipboard_text() -> Result<String, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    let text = clipboard.get_text().map_err(|e| e.to_string())?;
    Ok(text)
}
```

The `(async)` attribute moves execution off the main thread without forcing a rewrite to `async fn`. State access via `state.blocking_lock()` continues to work because the worker thread is blocking-friendly.

## When to apply

Audit any new command that:

- Touches the clipboard (`arboard`, `tauri-plugin-clipboard-manager`)
- Reads/writes files synchronously
- Spawns subprocesses and waits for them (`Command::output`, `wait`)
- Makes network calls (use `pub async fn` + `reqwest` instead)
- Calls into X11/GTK/Cocoa APIs that may block

Cheap rule of thumb: if the function body contains any I/O that isn't a fast in-memory operation, mark it `(async)`.

## Cleaner alternative when applicable

If the data is already available in the WebView (e.g. clipboard contents during a `paste` event), prefer reading it on the JS side via `e.clipboardData` instead of round-tripping through a Tauri command. WebKit's clipboard pipeline is already on the right thread and doesn't go through X11 selections — zero risk of the deadlock above. See [paste-handler.md](paste-handler.md).
