# File drag-and-drop into AI webviews — disable Tauri's OS drag-drop handler

## Rule

Every builder that hosts external AI content must call **`.disable_drag_drop_handler()`**:

- `services/ai_webview/lifecycle.rs` → `open_window` (`WebviewWindowBuilder`)
- `services/ai_webview/provider_swap.rs` → `hosted_swap_to_provider` (`WebviewBuilder` / `add_child`)

Keep it next to `.enable_clipboard_access()` on both.

## Why

By default Tauri installs an **OS-level drag-and-drop handler** on the webview. It
intercepts file drops at the native layer and turns them into Rust-side
`WindowEvent::DragDrop` events — the drop never reaches the page. So the hosted site's own
drop zone (e.g. Claude.ai project-creation "add documents", ChatGPT attachments) sees no
`dragenter`/`dragover`/`drop` and nothing happens.

`disable_drag_drop_handler()` turns the native interception off, so WebKitGTK/WebView2
delivers standard **HTML5 drag-and-drop** to the page and the site handles the file itself.
The method's own Tauri doc comment: *"This is required to use HTML5 drag and drop APIs on the
frontend."*

Trade-off: we lose Rust-side `DragDrop` events for these windows. We don't use them — the
hosted page owns file handling — so this is free.

## Symptom

Dragging a file onto the Claude/ChatGPT page does nothing: no drop highlight, no upload. A
file picker (click-to-browse) still works because that path never involved the drag handler.

## When to load this file

- Adding or modifying any `WebviewWindowBuilder` / `WebviewBuilder` that loads external
  provider content.
- Investigating "can't drop files into Claude/ChatGPT webview" reports.

## Related

- [paste-handler.md](paste-handler.md) — the sibling WebKitGTK clipboard/paste workarounds for these same webviews.
