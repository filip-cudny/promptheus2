# WebKit crash on NVIDIA Linux when a page enters accelerated compositing

## Rule

Keep `services/webkit_renderer.rs` called as the first statement of `app_lib::run()` (`lib.rs`, Linux only). On NVIDIA it sets `WEBKIT_FORCE_DMABUF_RENDERER=1` and `WEBKIT_DMABUF_RENDERER_FORCE_SHM=1` before any thread starts; do not move it into the Tauri setup hook or after GTK and WebKit initialize.

## Symptom

On GNOME Wayland with an NVIDIA GPU, logging in to chatgpt.com in the AI webview (e-mail or "Continue with Google") kills the app with SIGSEGV (apport log, no SIGABRT, so no Rust panic). The "freeze" is apport writing a multi-GB core for ~17 s while the windows stay on screen; the next crash vanishes at once.

## Why

- Ubuntu/Debian `disable-nvidia-dmabuf.patch` (WebKit bug 262607) returns early from `AcceleratedBackingStore::rendererBufferTransportMode()` when the GL vendor is NVIDIA, leaving the mode empty. Bypass: `WEBKIT_FORCE_DMABUF_RENDERER` set and not `0`.
- Empty mode: `checkRequirements()` false, hardware acceleration disabled, `AcceleratedBackingStore::create` returns null for every web view.
- The first page that enters accelerated compositing sends `EnterAcceleratedCompositingMode`; `AcceleratedBackingStore::update` dereferences the null backing store: SIGSEGV.
- Why the page enters accelerated compositing with acceleration off is not established (WebGL, enabled by wry in every web view, is a guess). The fix does not depend on it.
- Scope: Linux, NVIDIA GL vendor, a webkit2gtk build carrying the patch; any page in any web view. macOS (WKWebView) is unaffected.

## The fix

`services/webkit_renderer.rs::configure()`, called first in `lib.rs::run()`; the decision line is logged from the setup closure once the log plugin is up.

- NVIDIA detection: `/sys/module/nvidia` exists. Cost: a hybrid laptop with the module loaded but rendering on the integrated GPU also gets shared-memory transport (slower, not broken); a distro without the patch also gets it on NVIDIA. Nouveau does not create the path.
- Nothing is set when the user already set any of `WEBKIT_FORCE_DMABUF_RENDERER`, `WEBKIT_DMABUF_RENDERER_FORCE_SHM`, `WEBKIT_DISABLE_DMABUF_RENDERER`, `WEBKIT_DISABLE_COMPOSITING_MODE` (any value). A user with `WEBKIT_DISABLE_DMABUF_RENDERER=1` keeps the crash.
- Child processes inherit the variables; only WebKitGTK children react.
- Shared-memory transport costs a copy per frame instead of zero-copy DMA-BUF.

## Measurements

Same machine (NVIDIA RTX 3050 Ti, driver 595-open, Ubuntu 26.04, webkit2gtk 2.52.6-0ubuntu0.26.04.1, GNOME Wayland), installed app:

- `WEBKIT_DISABLE_DMABUF_RENDERER=1`: crash on login, same as without it (it empties the mode the same way).
- `WEBKIT_FORCE_DMABUF_RENDERER=1` alone (2 runs): the app exits on the first context-menu show, `Gdk-Message: Error 71 (Protocol error) dispatching to Wayland display`.
- `WEBKIT_FORCE_DMABUF_RENDERER=1 WEBKIT_DMABUF_RENDERER_FORCE_SHM=1`: context menu, chat and pill fine; the ChatGPT child loaded `auth.openai.com`, `accounts.google.com`, back to `chatgpt.com`; no crash.

Check the decision with the `WebKit renderer:` info line in the log.
