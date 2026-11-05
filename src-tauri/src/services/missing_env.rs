use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::models::settings::Settings;
use crate::services::env_resolve::env_ref_names;
use crate::services::notification::{NotificationLevel, NotificationService};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EnvUse {
    pub key: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MissingEnvRef {
    pub name: String,
    pub used_in: Vec<EnvUse>,
}

#[derive(Default)]
pub struct MissingEnvToastState(pub Mutex<BTreeSet<String>>);

pub fn env_is_set(name: &str) -> bool {
    crate::services::env_resolve::env_value(name).is_some_and(|v| !v.is_empty())
}

pub fn collect_missing(settings: &Settings, is_set: impl Fn(&str) -> bool) -> Vec<MissingEnvRef> {
    let mut found: BTreeMap<String, Vec<EnvUse>> = BTreeMap::new();
    let mut scan = |value: &str, key: String, label: String| {
        for name in env_ref_names(value) {
            if is_set(&name) {
                continue;
            }
            let uses = found.entry(name).or_default();
            if !uses.iter().any(|u| u.key == key) {
                uses.push(EnvUse {
                    key: key.clone(),
                    label: label.clone(),
                });
            }
        }
    };

    for model in &settings.models {
        if let Some(api_key) = &model.api_key {
            scan(
                api_key,
                format!("models[{}].api_key", model.id),
                format!("{} API key", model.display_name),
            );
        }
    }

    let servers: BTreeMap<_, _> = settings.mcp_servers.iter().collect();
    for (name, server) in servers {
        scan(
            &server.command,
            format!("mcp_servers.{name}.command"),
            format!("MCP server {name} command"),
        );
        for arg in &server.args {
            scan(
                arg,
                format!("mcp_servers.{name}.args"),
                format!("MCP server {name} arguments"),
            );
        }
        let env: BTreeMap<_, _> = server.env.iter().collect();
        for (var, value) in env {
            scan(
                value,
                format!("mcp_servers.{name}.env.{var}"),
                format!("MCP server {name} env {var}"),
            );
        }
    }

    let base = &settings.prompt_base;
    scan(&base.system, "prompt_base.system".into(), "System prompt file".into());
    scan(&base.about_you, "prompt_base.about_you".into(), "About you file".into());
    scan(&base.environment, "prompt_base.environment".into(), "Environment file".into());
    scan(&base.input_format, "prompt_base.input_format".into(), "Input format file".into());
    scan(
        &settings.surfaces.title_generation.prompt,
        "surfaces.title_generation.prompt".into(),
        "Title generation prompt file".into(),
    );
    let stt = &settings.surfaces.speech_to_text;
    for (value, key, label) in [
        (&stt.prompt, "surfaces.speech_to_text.prompt", "Speech to text prompt file"),
        (&stt.keyterms_file, "surfaces.speech_to_text.keyterms_file", "Speech to text keyterms file"),
    ] {
        if let Some(value) = value {
            scan(value, key.into(), label.into());
        }
    }

    found
        .into_iter()
        .map(|(name, used_in)| MissingEnvRef { name, used_in })
        .collect()
}

pub fn should_toast(last_shown: &BTreeSet<String>, current: &BTreeSet<String>) -> bool {
    current.iter().any(|name| !last_shown.contains(name))
}

pub fn check_and_toast(app: &AppHandle, settings: &Settings) {
    let current: BTreeSet<String> = collect_missing(settings, env_is_set)
        .into_iter()
        .map(|m| m.name)
        .collect();
    let state = app.state::<MissingEnvToastState>();
    let mut last_shown = state.0.lock().unwrap_or_else(|e| e.into_inner());
    if !should_toast(&last_shown, &current) {
        log::debug!("missing env refs: {} name(s), no toast", current.len());
        return;
    }
    let names = current.iter().cloned().collect::<Vec<_>>().join(", ");
    log::warn!("environment variables without a value: {names}");
    app.state::<NotificationService>().notify_with_action(
        "missing_env_refs",
        NotificationLevel::Warning,
        "Missing environment variables",
        Some(format!("{names} — click to open Settings")),
        Some("open_settings"),
        settings,
    );
    *last_shown = current;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::settings::{McpServerConfig, ModelConfig};
    use std::collections::HashMap;

    fn set(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    #[test]
    fn collects_only_unset_names_with_their_uses() {
        let mut settings = Settings::default();
        settings.models.push(
            serde_json::from_str::<ModelConfig>(
                r#"{"id":"m1","model":"x","display_name":"M","api_key":"${SET_A}"}"#,
            )
            .unwrap(),
        );
        settings.mcp_servers.insert(
            "srv".into(),
            McpServerConfig {
                command: "${UNSET_B}/bin/x".into(),
                args: vec!["${UNSET_B}".into()],
                env: HashMap::from([("K".into(), "${UNSET_C}".into())]),
                timeout_secs: 180,
                tool_timeouts: HashMap::new(),
            },
        );
        settings.prompt_base.system = "${UNSET_D}/system.md".into();

        let missing = collect_missing(&settings, |name| name == "SET_A");

        let names: Vec<_> = missing.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, vec!["UNSET_B", "UNSET_C", "UNSET_D"]);
        let keys: Vec<_> = missing[0].used_in.iter().map(|u| u.key.as_str()).collect();
        assert_eq!(keys, vec!["mcp_servers.srv.command", "mcp_servers.srv.args"]);
    }

    #[test]
    fn toasts_only_when_a_name_is_new() {
        assert!(should_toast(&set(&[]), &set(&["A"])));
        assert!(!should_toast(&set(&["A"]), &set(&["A"])));
        assert!(!should_toast(&set(&["A", "B"]), &set(&["A"])));
        assert!(should_toast(&set(&["A"]), &set(&["A", "B"])));
    }
}
