use crate::models::settings::WebviewProvider;

use super::provider_swap::ROUTER_SENTINEL;

pub(super) fn initialization_script(provider: &WebviewProvider) -> String {
    let provider_id_json =
        serde_json::to_string(&provider.id).unwrap_or_else(|_| "\"\"".to_string());
    let sentinel_json =
        serde_json::to_string(ROUTER_SENTINEL).unwrap_or_else(|_| "\"\"".to_string());

    format!(
        r#"
        {dark_mode}
        (function() {{
            if (window.__promptheus && window.__promptheus.__installed) return;
            const g = window.__promptheus = window.__promptheus || {{}};
            g.__installed = true;
            g.providerId = {provider_id_json};
            g.routerSentinel = {sentinel_json};
            {palette}
            {external_links}
            {clipboard_paste_fallback}
            {deliver_message}
        }})();
        "#,
        provider_id_json = provider_id_json,
        sentinel_json = sentinel_json,
        palette = PALETTE_KEYBIND_JS,
        external_links = EXTERNAL_LINKS_JS,
        clipboard_paste_fallback = CLIPBOARD_PASTE_FALLBACK_JS,
        deliver_message = DELIVER_MESSAGE_JS,
        dark_mode = DARK_MODE_JS,
    )
}

pub(super) fn delivery_script(
    run: u64,
    transcript: &str,
    context_text: Option<&str>,
    images: &[(String, String)],
) -> String {
    let text = match context_text {
        Some(context) => format!("{context}\n\n{transcript}"),
        None => transcript.to_string(),
    };
    let images: Vec<_> = images
        .iter()
        .map(|(media_type, data)| serde_json::json!({ "mediaType": media_type, "data": data }))
        .collect();
    let payload = serde_json::json!({ "run": run, "text": text, "images": images });
    let payload = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
    format!(
        "window.__promptheus && window.__promptheus.__deliverMessage && window.__promptheus.__deliverMessage({payload});"
    )
}

pub(super) fn reinject_script() -> String {
    r#"
    (function() {
        if (!window.__promptheus) return;
        if (window.__promptheus.ensurePaletteKeybind) {
            window.__promptheus.ensurePaletteKeybind();
        }
        if (window.__promptheus.ensureExternalLinks) {
            window.__promptheus.ensureExternalLinks();
        }
        if (window.__promptheus.ensureClipboardPasteFallback) {
            window.__promptheus.ensureClipboardPasteFallback();
        }
    })();
    "#
    .to_string()
}

const DARK_MODE_JS: &str = r##"
(function() {
    try {
        const native = window.matchMedia ? window.matchMedia.bind(window) : null;
        if (native) {
            window.matchMedia = function(query) {
                const result = native(query);
                if (typeof query === "string" && query.indexOf("prefers-color-scheme") !== -1) {
                    const wantsDark = query.indexOf("dark") !== -1;
                    try {
                        Object.defineProperty(result, "matches", { value: wantsDark, configurable: true });
                    } catch (_) {}
                }
                return result;
            };
        }
    } catch (_) {}
    try {
        if (document.documentElement) {
            document.documentElement.style.colorScheme = "dark";
        }
    } catch (_) {}
    try {
        if (window.localStorage) {
            window.localStorage.setItem("theme", "dark");
        }
    } catch (_) {}
})();
"##;

const PALETTE_KEYBIND_JS: &str = r##"
const S = g.routerSentinel;

function sendRouter(params) {
    const qs = new URLSearchParams(params).toString();
    window.location.href = S + "?" + qs;
}

function paletteKeydown(e) {
    if (!(e.metaKey || e.ctrlKey)) return;
    if (e.shiftKey || e.altKey) return;
    const key = typeof e.key === "string" ? e.key.toLowerCase() : "";
    if (key !== "p") return;
    e.preventDefault();
    e.stopPropagation();
    sendRouter({ kind: "open_palette" });
}

function ensurePaletteKeybind() {
    if (g.__paletteInstalled) return;
    g.__paletteInstalled = true;
    document.addEventListener("keydown", paletteKeydown, true);
    window.addEventListener("keydown", paletteKeydown, true);
}

g.ensurePaletteKeybind = ensurePaletteKeybind;
ensurePaletteKeybind();
"##;

const EXTERNAL_LINKS_JS: &str = r##"
const origOpen = window.open ? window.open.bind(window) : null;

function isCrossOrigin(targetUrl) {
    try {
        const u = new URL(targetUrl, location.href);
        if (u.protocol !== "http:" && u.protocol !== "https:") return false;
        return u.host !== location.host;
    } catch (_) {
        return false;
    }
}

function externalClick(e) {
    if (e.defaultPrevented) return;
    if (e.button === 2) return;
    const a = e.target && e.target.closest && e.target.closest("a[href]");
    if (!a) return;
    const href = a.href;
    if (!href) return;
    const isBlank = a.target === "_blank";
    const cross = isCrossOrigin(href);
    if (!isBlank && !cross) return;
    e.preventDefault();
    e.stopPropagation();
    sendRouter({ kind: "open_external", url: href });
}

function patchedOpen(url, target, features) {
    try {
        if (typeof url === "string" && isCrossOrigin(url)) {
            sendRouter({ kind: "open_external", url: new URL(url, location.href).href });
            return null;
        }
    } catch (_) {}
    return origOpen ? origOpen(url, target, features) : null;
}

function ensureExternalLinks() {
    if (g.__externalLinksInstalled) return;
    g.__externalLinksInstalled = true;
    document.addEventListener("click", externalClick, true);
    document.addEventListener("auxclick", externalClick, true);
    if (origOpen) window.open = patchedOpen;
}

g.ensureExternalLinks = ensureExternalLinks;
ensureExternalLinks();
"##;

const CLIPBOARD_PASTE_FALLBACK_JS: &str = r##"
function dispatchSyntheticPaste(target, dt) {
    const evt = new ClipboardEvent("paste", { clipboardData: dt, bubbles: true, cancelable: true });
    (target || document.activeElement || document.body).dispatchEvent(evt);
}

function injectImage(target, blob) {
    const dt = new DataTransfer();
    dt.items.add(new File([blob], "pasted-image", { type: blob.type }));
    dispatchSyntheticPaste(target, dt);
}

function insertTextIntoEditable(el, text) {
    if (!el) return;
    const tag = el.tagName;
    if (tag === "TEXTAREA" || tag === "INPUT") {
        try {
            const start = el.selectionStart != null ? el.selectionStart : el.value.length;
            const end = el.selectionEnd != null ? el.selectionEnd : el.value.length;
            if (typeof el.setRangeText === "function") {
                el.setRangeText(text, start, end, "end");
            } else {
                el.value = el.value.slice(0, start) + text + el.value.slice(end);
            }
            el.dispatchEvent(new Event("input", { bubbles: true }));
        } catch (_) {}
        return;
    }
    if (el.isContentEditable) {
        document.execCommand("insertText", false, text);
    }
}

function injectText(target, text) {
    const dt = new DataTransfer();
    dt.setData("text/plain", text);
    const el = target || document.activeElement || document.body;
    const evt = new ClipboardEvent("paste", { clipboardData: dt, bubbles: true, cancelable: true });
    const notConsumed = el.dispatchEvent(evt);
    if (notConsumed) insertTextIntoEditable(el, text);
}

g.__deliverPasteText = function(text) {
    const target = g.__pasteTarget || document.activeElement || document.body;
    g.__pasteTarget = null;
    if (text) injectText(target, text);
};

function requestHostText(target) {
    g.__pasteTarget = target;
    sendRouter({ kind: "request_paste_text" });
}

function clipboardPasteFallback(e) {
    const dt = e.clipboardData;
    if (dt && dt.files && dt.files.length > 0) return;
    const types = dt ? Array.prototype.slice.call(dt.types || []) : [];
    if (types.some(function(t) { return t.indexOf("text/") === 0; })) return;
    if (!(navigator.clipboard && navigator.clipboard.read)) return;

    e.preventDefault();
    e.stopImmediatePropagation();
    const target = e.target;

    navigator.clipboard.read().then(function(clipItems) {
        for (let i = 0; i < clipItems.length; i++) {
            const ci = clipItems[i];
            const imgType = ci.types.find(function(t) { return t.indexOf("image/") === 0; });
            if (imgType) {
                ci.getType(imgType).then(function(blob) { injectImage(target, blob); }).catch(function() {});
                return;
            }
        }
        requestHostText(target);
    }).catch(function() {
        requestHostText(target);
    });
}

function ensureClipboardPasteFallback() {
    if (g.__clipboardPasteFallbackInstalled) return;
    g.__clipboardPasteFallbackInstalled = true;
    document.addEventListener("paste", clipboardPasteFallback, true);
}

g.ensureClipboardPasteFallback = ensureClipboardPasteFallback;
ensureClipboardPasteFallback();
"##;

const DELIVER_MESSAGE_JS: &str = r##"
function decodeImage(image) {
    const bin = atob(image.data);
    const bytes = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
    return new Blob([bytes], { type: image.mediaType });
}

function pressEnter(composer) {
    composer.dispatchEvent(new KeyboardEvent("keydown", {
        key: "Enter", code: "Enter", keyCode: 13, which: 13, bubbles: true, cancelable: true,
    }));
}

let uploadCounter = null;

function countBodyImages(body) {
    if (body instanceof Blob) return 1;
    if (body instanceof FormData) {
        let n = 0;
        body.forEach(function(value) { if (value instanceof Blob) n++; });
        return n;
    }
    return 0;
}

function trackUpload(body) {
    const counter = uploadCounter;
    const count = counter ? countBodyImages(body) : 0;
    if (!count) return null;
    counter.images += count;
    counter.inFlight += 1;
    counter.lastAt = Date.now();
    return function() {
        counter.inFlight -= 1;
        counter.lastAt = Date.now();
    };
}

if (window.fetch) {
    const origFetch = window.fetch;
    window.fetch = function(input, init) {
        const settle = init && init.keepalive !== true ? trackUpload(init.body) : null;
        let promise;
        try {
            promise = origFetch.apply(this, arguments);
        } catch (e) {
            if (settle) settle();
            throw e;
        }
        if (settle) promise.then(settle, settle);
        return promise;
    };
}

const origXhrSend = XMLHttpRequest.prototype.send;
XMLHttpRequest.prototype.send = function(body) {
    const settle = trackUpload(body);
    if (settle) this.addEventListener("loadend", settle);
    try {
        return origXhrSend.apply(this, arguments);
    } catch (e) {
        if (settle) settle();
        throw e;
    }
};

function sendUntilNavigated(composer, run, uploads, waitedMs) {
    const startedAt = Date.now();
    pressEnter(composer);
    const timer = setInterval(function() {
        if (/^\/(chat|c)\//.test(location.pathname)) {
            clearInterval(timer);
            sendRouter({ kind: "webview_send_outcome", status: "sent", run: run, uploads: uploads, waitedMs: waitedMs });
        } else if (Date.now() - startedAt >= 30000) {
            clearInterval(timer);
            sendRouter({ kind: "webview_send_outcome", status: "typed", run: run, uploads: uploads, waitedMs: waitedMs });
        } else {
            pressEnter(composer);
        }
    }, 500);
}

g.__deliverMessage = function(payload) {
    const startedAt = Date.now();
    const poll = setInterval(function() {
        const composer = document.querySelector('.ProseMirror[contenteditable="true"]');
        if (!composer) {
            if (Date.now() - startedAt >= 15000) {
                clearInterval(poll);
                sendRouter({ kind: "webview_send_outcome", status: "no_composer", run: payload.run });
            }
            return;
        }
        clearInterval(poll);
        composer.focus();
        if (!payload.images.length) {
            injectText(composer, payload.text);
            sendUntilNavigated(composer, payload.run, 0, 0);
            return;
        }
        const counter = { images: 0, inFlight: 0, lastAt: Date.now() };
        const uploadStartedAt = Date.now();
        uploadCounter = counter;
        payload.images.forEach(function(image) { injectImage(composer, decodeImage(image)); });
        const wait = setInterval(function() {
            const now = Date.now();
            const done = counter.images >= payload.images.length
                && counter.inFlight === 0 && now - counter.lastAt >= 1000;
            const timedOut = now - uploadStartedAt >= 20000;
            if (!done && !timedOut) return;
            clearInterval(wait);
            uploadCounter = null;
            const waitedMs = now - uploadStartedAt;
            injectText(composer, payload.text);
            if (done) {
                sendUntilNavigated(composer, payload.run, counter.images, waitedMs);
            } else {
                sendRouter({ kind: "webview_send_outcome", status: "typed", run: payload.run, uploads: counter.images, waitedMs: waitedMs });
            }
        }, 100);
    }, 250);
};
"##;

#[cfg(test)]
mod tests {
    use super::*;

    fn provider() -> WebviewProvider {
        WebviewProvider {
            id: "claude".into(),
            name: "Claude".into(),
            url: "https://claude.ai/".into(),
        }
    }

    #[test]
    fn initialization_script_embeds_provider_id() {
        let js = initialization_script(&provider());
        assert!(js.contains("\"claude\""));
        assert!(js.contains(ROUTER_SENTINEL));
    }

    #[test]
    fn initialization_script_includes_palette_keybind() {
        let js = initialization_script(&provider());
        assert!(js.contains("ensurePaletteKeybind"));
    }

    #[test]
    fn initialization_script_includes_dark_mode_shim() {
        let js = initialization_script(&provider());
        assert!(js.contains("colorScheme"));
        assert!(js.contains("prefers-color-scheme"));
    }

    #[test]
    fn reinject_script_is_idempotent_guard() {
        let js = reinject_script();
        assert!(js.contains("if (!window.__promptheus"));
        assert!(js.contains("ensurePaletteKeybind"));
        assert!(js.contains("ensureExternalLinks"));
    }

    #[test]
    fn initialization_script_includes_external_links_shim() {
        let js = initialization_script(&provider());
        assert!(js.contains("ensureExternalLinks"));
        assert!(js.contains("open_external"));
        assert!(js.contains("auxclick"));
    }

    #[test]
    fn initialization_script_includes_clipboard_paste_fallback() {
        let js = initialization_script(&provider());
        assert!(js.contains("ensureClipboardPasteFallback"));
        assert!(js.contains("navigator.clipboard.read"));
        assert!(js.contains("ClipboardEvent"));
        assert!(js.contains("request_paste_text"));
        assert!(js.contains("__deliverPasteText"));
    }

    #[test]
    fn reinject_script_reinstalls_clipboard_paste_fallback() {
        let js = reinject_script();
        assert!(js.contains("ensureClipboardPasteFallback"));
    }

    #[test]
    fn delivery_script_puts_context_before_transcript() {
        let js = delivery_script(3, "say", Some("ctx"), &[]);
        assert!(js.contains(&serde_json::to_string("ctx\n\nsay").unwrap()));
        let js = delivery_script(3, "say", None, &[]);
        assert!(js.contains(&serde_json::to_string("say").unwrap()));
    }

    #[test]
    fn delivery_script_escapes_transcript_like_serde_json() {
        let transcript = "a \"q\"\n</script>";
        let js = delivery_script(1, transcript, None, &[]);
        assert!(js.contains(&serde_json::to_string(transcript).unwrap()));
    }

    #[test]
    fn delivery_script_carries_images() {
        let js = delivery_script(1, "t", None, &[("image/png".into(), "QUJD".into())]);
        assert!(js.contains(r#""mediaType":"image/png""#));
        assert!(js.contains(r#""data":"QUJD""#));
    }

    #[test]
    fn initialization_script_includes_deliver_message() {
        assert!(initialization_script(&provider()).contains("__deliverMessage"));
    }
}
