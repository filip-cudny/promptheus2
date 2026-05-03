const CAPABILITY_FILES: &[(&str, &str)] = &[(
    "default.json",
    include_str!("../../capabilities/default.json"),
)];

pub(crate) fn assert_no_ai_webview_ipc_grant() {
    for (filename, contents) in CAPABILITY_FILES {
        let value: serde_json::Value = serde_json::from_str(contents)
            .unwrap_or_else(|e| panic!("capabilities/{filename} must be valid JSON: {e}"));
        for key in ["windows", "webviews"] {
            audit_capability_section(filename, &value, key);
        }
        if capability_label_list(&value, "windows")
            .iter()
            .any(|l| *l == "conversation-dialog")
        {
            panic!(
                "capabilities/{filename} lists 'conversation-dialog' in windows[]; \
                 it must live in webviews[] only — otherwise provider child webviews inherit IPC."
            );
        }
    }
}

fn capability_label_list<'a>(value: &'a serde_json::Value, key: &str) -> Vec<&'a str> {
    value
        .get(key)
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default()
}

fn audit_capability_section(filename: &str, value: &serde_json::Value, key: &str) {
    for label in capability_label_list(value, key) {
        if label == "*" {
            panic!(
                "capabilities/{filename} grants IPC to every {key} via '*'; \
                 list specific labels instead."
            );
        }
        if label.contains("ai-webview-") {
            panic!(
                "capabilities/{filename} grants IPC to provider webview '{label}' via {key}[]; \
                 provider webviews must never have IPC access."
            );
        }
    }
}
