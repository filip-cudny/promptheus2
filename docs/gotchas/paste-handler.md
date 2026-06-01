# Shift+Cmd/Ctrl+V "paste raw text" — platform split

## Rule

The manual `Shift+Cmd+V` keydown branch in `src/lib/components/prompt/InputArea.svelte → handleKeydown` must be gated on **`isMac && e.metaKey`**, never `(e.metaKey || e.ctrlKey)`. On Linux/Windows the native browser `paste` event must run instead. This has ping-ponged between platforms multiple times.

## Why each platform needs a different path

**macOS:** WebKit doesn't reliably fire a usable `paste` event for `Cmd+Shift+V`. That shortcut is system-bound to "Paste and Match Style"; depending on WebKit version, either no `paste` event reaches JS, or the event arrives with already-stripped `clipboardData` that we can't override. The manual `invoke("get_clipboard_text")` path uses NSPasteboard via `arboard` — fast, reliable, no deadlock risk (NSPasteboard is brokered, not peer-to-peer).

**Linux/Windows:** The browser fires a normal `paste` event for `Shift+Ctrl+V`. `handleEditablePaste` already reads `e.clipboardData.getData("text/plain")` (instant, in-memory, no IPC, no X11) and honors `skipTextAttachment: shiftHeld` — exactly the desired raw-text behavior. Calling `arboard` from a Linux Tauri command re-introduces a self-deadlock with the WebKit webview's own X11 selection ownership, which can freeze the app for many seconds. See [tauri-command-threading.md](tauri-command-threading.md).

## What the code looks like

```ts
const isMac = typeof navigator !== "undefined" && /Mac/.test(navigator.platform);

async function handleKeydown(e: KeyboardEvent) {
  if (isMac && e.key.toLowerCase() === "v" && e.shiftKey && e.metaKey) {
    e.preventDefault();
    try {
      const text = await invoke<string>("get_clipboard_text");
      if (text) { document.execCommand("insertText", false, text); return; }
    } catch {}
    try {
      const [data, mediaType] = await invoke<[string, string]>("get_clipboard_image");
      if (data) localImages = [...localImages, { data, media_type: mediaType }];
    } catch {}
    return;
  }
  // ... rest of handler; native paste event handles Linux/Windows
}
```

## Failure modes (look for these when changing the handler)

- **Linux freeze on Shift+Ctrl+V**: the gate was loosened to include `e.ctrlKey`. Fix: re-tighten to `isMac && e.metaKey`.
- **Mac pastes nothing on Cmd+Shift+V**: the gate dropped the manual branch entirely. Fix: keep the `isMac` branch with the `arboard` invoke.
- **Mac pastes a "text attachment" instead of inline raw text**: `skipTextAttachment` not propagated. The manual macOS branch already inserts directly via `execCommand`, so this only matters for the Linux native-paste path — verify `handleEditablePaste` is called with `skipTextAttachment: shiftHeld`.

## Empty-DataTransfer paste into AI webviews (Claude.ai / ChatGPT) — WebKitGTK workaround

Separate problem from the native dialog: pasting into a hosted external webview
(`*::ai-webview-*`) on Linux can silently do nothing. Affects **images** always, and
**text that an in-process producer wrote via `arboard`** (e.g. the Shift+F1 transcription
result) — both arrive with an empty payload. Text copied in another app or inside the
webview is unaffected.

**Cause.** On WebKitGTK the legacy synchronous paste path delivers an **empty
`DataTransfer`** — the page's `paste` handler sees `types=[] items=[] files=[]`
(WebKit bug 218519). The site's own paste code reads `e.clipboardData` (`.files` for
images, `getData("text/plain")` for text), gets nothing, and aborts. The system
`libwebkit2gtk` 2.52.x carries the upstream fix yet the embedded wry webview still
delivers nothing on this path; it is especially reproducible when our own process owns the
X11 selection. The async API behaves differently per source:
- **External clipboard** (image/text copied in another app): `navigator.clipboard.read()`
  returns it fine.
- **Same-process clipboard** (text we wrote via `xclip`/`wl-copy`/arboard — e.g. the
  Shift+F1 transcription result): the embedded WebKit returns **empty** from both
  `read()` and `readText()`. It cannot read a selection our own process owns. This is why
  the JS-only async read is insufficient and the host bridge below exists.

**Fix** (injected script, `services/ai_webview/scripts.rs → CLIPBOARD_PASTE_FALLBACK_JS`):
a capture-phase `paste` listener detects an empty payload (no `files` **and** no `text/*`
type), then:
- **image** → `navigator.clipboard.read()` finds the `image/*`, builds a `File`, and
  re-dispatches a synthetic `paste` with populated `clipboardData`.
- **no image** (our-process text) → asks the host via the router sentinel
  (`kind=request_paste_text`). Rust reads the clipboard with `ClipboardService::get_text()`
  (arboard) on the `spawn_blocking` router worker — **off** the GTK main thread, so no X11
  self-deadlock — and `eval`s `window.__promptheus.__deliverPasteText(text)` back into the
  webview, which re-dispatches a synthetic `text/plain` paste.

Both paths end in a synthetic `paste` `ClipboardEvent` hitting the same handler path that
works on Chrome/Firefox. Site-agnostic (works for Claude and ChatGPT).

**Guards — do not remove:**
- Skip when `e.clipboardData.files.length > 0` (image already delivered, e.g. macOS/Windows).
- Skip when any `text/*` type is present (normal text paste — leave it alone).
- The synthetic event carries `files` (image) or a `text/plain` type (text), so the listener
  short-circuits on its own re-dispatch (no loop).
- The host bridge reads the clipboard on the router's `spawn_blocking` worker, never on a
  sync `#[tauri::command]` / the GTK main thread — keep it that way (see
  [tauri-command-threading.md](tauri-command-threading.md)).

**Why this does not reintroduce the X11 arboard deadlock:** the image path reads on the JS
side via WebKit's own clipboard pipeline (no `arboard` at all); the text path reads
`arboard` only on the router's `spawn_blocking` worker, never on a sync `#[tauri::command]`
or the GTK main thread. Neither issues an X11 `SelectionRequest` from the main loop — this
is the "Cleaner alternative" in [tauri-command-threading.md](tauri-command-threading.md).

The JS `navigator.clipboard.read()` (image path) requires `enable_clipboard_access()` on
the webview builders (`services/ai_webview/lifecycle.rs`, `provider_swap.rs`). No
`permission-request` handling is needed: that signal never fires for clipboard in this
embedded webview, and `enable_clipboard_access()` alone unlocks `read()`.

## When to load this file

- Touching `InputArea.svelte → handleKeydown` or its paste/keydown logic.
- Touching `src/lib/utils/paste.ts` (`handleEditablePaste`, `getImageFromPasteEvent`).
- Touching any Tauri clipboard command (`get_clipboard_text`, `get_clipboard_image`, `set_clipboard_text`).
- Touching `services/ai_webview/scripts.rs` paste injection or the `request_paste_text` host bridge in `palette.rs`.
- Investigating "image/text paste does nothing in Claude/ChatGPT webview", "transcription result won't paste", or any "paste freezes" / "paste does nothing" report.

## Related

- [tauri-command-threading.md](tauri-command-threading.md) — why the Linux `arboard` path freezes the app and why the backend commands are now `(async)`.
