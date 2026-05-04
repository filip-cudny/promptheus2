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
        }})();
        "#,
        provider_id_json = provider_id_json,
        sentinel_json = sentinel_json,
        palette = PALETTE_KEYBIND_JS,
        dark_mode = DARK_MODE_JS,
    )
}

pub(super) fn reinject_script() -> String {
    r#"
    (function() {
        if (!window.__promptheus || !window.__promptheus.ensurePaletteKeybind) return;
        window.__promptheus.ensurePaletteKeybind();
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
    }
}
