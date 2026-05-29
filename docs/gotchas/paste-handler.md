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

## Image paste into AI webviews (Claude.ai / ChatGPT) — WebKitGTK workaround

Separate problem from the native dialog: pasting an **image** into a hosted external
webview (`*::ai-webview-*`) silently does nothing on Linux, while text pastes fine.

**Cause.** On WebKitGTK the legacy synchronous paste path delivers an **empty
`DataTransfer`** for images — the page's `paste` handler sees `types=[] items=[] files=[]`
(WebKit bug 218519). The site's own image-paste code reads `e.clipboardData.files`, gets
nothing, and aborts. The system `libwebkit2gtk` 2.52.x carries the upstream fix yet the
embedded wry webview still delivers nothing on this path. The **async** Clipboard API is
unaffected: `navigator.clipboard.read()` returns the `image/png` correctly.

**Fix** (injected script, `services/ai_webview/scripts.rs → CLIPBOARD_IMAGE_PASTE_JS`):
a capture-phase `paste` listener detects an empty payload (no `files` **and** no `text/*`
type — i.e. an image WebKit dropped), reads the image via `navigator.clipboard.read()`,
builds a `File` + `DataTransfer`, and re-dispatches a synthetic `paste` `ClipboardEvent`
with populated `clipboardData` to the editor — hitting the same handler path that works on
Chrome/Firefox. Site-agnostic (works for Claude and ChatGPT).

**Guards — do not remove:**
- Skip when `e.clipboardData.files.length > 0` (image already delivered, e.g. macOS/Windows).
- Skip when any `text/*` type is present (normal text paste — leave it alone).
- The synthetic event carries `files`, so the listener short-circuits on it (no loop).

**Why this does not reintroduce the X11 arboard deadlock:** the read happens on the JS
side via WebKit's own clipboard pipeline — no `arboard`, no sync Tauri command, no X11
`SelectionRequest` from the GTK main thread. This is exactly the "Cleaner alternative" in
[tauri-command-threading.md](tauri-command-threading.md). Requires
`enable_clipboard_access()` on the webview builders and allowing
`WebKitClipboardPermissionRequest` in the `permission-request` handler
(`services/ai_webview/lifecycle.rs`), since `clipboard.read()` needs permission.

## When to load this file

- Touching `InputArea.svelte → handleKeydown` or its paste/keydown logic.
- Touching `src/lib/utils/paste.ts` (`handleEditablePaste`, `getImageFromPasteEvent`).
- Touching any Tauri clipboard command (`get_clipboard_text`, `get_clipboard_image`, `set_clipboard_text`).
- Touching `services/ai_webview/scripts.rs` paste injection or the `permission-request` handler in `lifecycle.rs`.
- Investigating "image paste does nothing in Claude/ChatGPT webview" or any "paste freezes" / "paste does nothing" report.

## Related

- [tauri-command-threading.md](tauri-command-threading.md) — why the Linux `arboard` path freezes the app and why the backend commands are now `(async)`.
