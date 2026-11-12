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

pub fn visit_refs(settings: &Settings, mut visit: impl FnMut(&str, String, String)) {
    for model in &settings.models {
        if let Some(api_key) = &model.api_key {
            visit(
                api_key,
                format!("models[{}].api_key", model.id),
                format!("{} API key", model.display_name),
            );
        }
    }

    let servers: BTreeMap<_, _> = settings.mcp_servers.iter().collect();
    for (name, server) in servers {
        visit(
            &server.command,
            format!("mcp_servers.{name}.command"),
            format!("MCP server {name} command"),
        );
        for arg in &server.args {
            visit(
                arg,
                format!("mcp_servers.{name}.args"),
                format!("MCP server {name} arguments"),
            );
        }
        let env: BTreeMap<_, _> = server.env.iter().collect();
        for (var, value) in env {
            visit(
                value,
                format!("mcp_servers.{name}.env.{var}"),
                format!("MCP server {name} env {var}"),
            );
        }
    }

    let base = &settings.prompt_base;
    visit(&base.system, "prompt_base.system".into(), "System prompt file".into());
    visit(&base.about_you, "prompt_base.about_you".into(), "About you file".into());
    visit(&base.environment, "prompt_base.environment".into(), "Environment file".into());
    visit(&base.input_format, "prompt_base.input_format".into(), "Input format file".into());
    visit(
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
            visit(value, key.into(), label.into());
        }
    }
}

pub fn visit_refs_mut(settings: &mut Settings, mut visit: impl FnMut(&mut String)) {
    for model in &mut settings.models {
        if let Some(api_key) = &mut model.api_key {
            visit(api_key);
        }
    }

    let mut servers: Vec<_> = settings.mcp_servers.iter_mut().collect();
    servers.sort_by(|a, b| a.0.cmp(b.0));
    for (_, server) in servers {
        visit(&mut server.command);
        for arg in &mut server.args {
            visit(arg);
        }
        let mut env: Vec<_> = server.env.iter_mut().collect();
        env.sort_by(|a, b| a.0.cmp(b.0));
        for (_, value) in env {
            visit(value);
        }
    }

    let base = &mut settings.prompt_base;
    visit(&mut base.system);
    visit(&mut base.about_you);
    visit(&mut base.environment);
    visit(&mut base.input_format);
    visit(&mut settings.surfaces.title_generation.prompt);
    let stt = &mut settings.surfaces.speech_to_text;
    for value in [&mut stt.prompt, &mut stt.keyterms_file] {
        if let Some(value) = value {
            visit(value);
        }
    }
}

pub fn collect_missing(settings: &Settings, is_set: impl Fn(&str) -> bool) -> Vec<MissingEnvRef> {
    let mut found: BTreeMap<String, Vec<EnvUse>> = BTreeMap::new();
    visit_refs(settings, |value, key, label| {
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
    });
    found
        .into_iter()
        .map(|(name, used_in)| MissingEnvRef { name, used_in })
        .collect()
}

pub fn collect_missing_enabled(settings: &Settings, is_set: impl Fn(&str) -> bool) -> Vec<MissingEnvRef> {
    let disabled_prefixes: Vec<String> = settings
        .mcp_servers
        .iter()
        .filter(|(_, server)| server.disabled)
        .map(|(name, _)| format!("mcp_servers.{name}."))
        .collect();
    collect_missing(settings, is_set)
        .into_iter()
        .filter_map(|mut missing| {
            missing
                .used_in
                .retain(|u| !disabled_prefixes.iter().any(|prefix| u.key.starts_with(prefix)));
            (!missing.used_in.is_empty()).then_some(missing)
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EnvVarStatus {
    pub name: String,
    pub description: String,
    pub secret: bool,
    pub set: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    pub used_in: Vec<EnvUse>,
}

pub fn env_var_overview(
    settings: &Settings,
    is_set: impl Fn(&str) -> bool,
    value_of: impl Fn(&str) -> Option<String>,
) -> Vec<EnvVarStatus> {
    let mut uses: BTreeMap<String, Vec<EnvUse>> = collect_missing(settings, |_| false)
        .into_iter()
        .map(|m| (m.name, m.used_in))
        .collect();
    settings
        .env_vars
        .iter()
        .map(|var| EnvVarStatus {
            name: var.name.clone(),
            description: var.description.clone(),
            secret: var.secret,
            set: is_set(&var.name),
            value: if var.secret { None } else { value_of(&var.name).filter(|value| !value.is_empty()) },
            used_in: uses.remove(&var.name).unwrap_or_default(),
        })
        .collect()
}

pub fn should_toast(last_shown: &BTreeSet<String>, current: &BTreeSet<String>) -> bool {
    current.iter().any(|name| !last_shown.contains(name))
}

pub fn check_and_toast(app: &AppHandle, settings: &Settings) {
    let current: BTreeSet<String> = collect_missing_enabled(settings, env_is_set)
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
                ..Default::default()
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
    fn both_walkers_visit_the_same_fields_in_the_same_order() {
        let mut settings = Settings::default();
        settings.models.push(
            serde_json::from_str::<ModelConfig>(
                r#"{"id":"m1","model":"x","display_name":"M","api_key":"key"}"#,
            )
            .unwrap(),
        );
        settings.mcp_servers.insert(
            "b".into(),
            McpServerConfig {
                command: "b-cmd".into(),
                args: vec!["b-arg".into()],
                env: HashMap::from([("Z".into(), "b-z".into()), ("A".into(), "b-a".into())]),
                timeout_secs: 180,
                tool_timeouts: HashMap::new(),
                ..Default::default()
            },
        );
        settings.mcp_servers.insert(
            "a".into(),
            McpServerConfig {
                command: "a-cmd".into(),
                args: vec![],
                env: HashMap::new(),
                timeout_secs: 180,
                tool_timeouts: HashMap::new(),
                ..Default::default()
            },
        );
        settings.surfaces.speech_to_text.prompt = Some("stt-prompt".into());
        settings.surfaces.speech_to_text.keyterms_file = Some("stt-terms".into());
        let mut read = Vec::new();
        visit_refs(&settings, |value, _, _| read.push(value.to_string()));

        let mut written = Vec::new();
        visit_refs_mut(&mut settings, |value| written.push(value.clone()));

        assert_eq!(read, written);
        assert_eq!(read.len(), 13);
        assert_eq!(read[..6], ["key", "a-cmd", "b-cmd", "b-arg", "b-a", "b-z"]);
    }

    #[test]
    fn off_server_env_is_not_missing_but_still_collected() {
        let mut settings = Settings::default();
        settings.mcp_servers.insert(
            "off".into(),
            McpServerConfig {
                command: "x".into(),
                env: HashMap::from([("K".into(), "${OFF_VAR}".into())]),
                disabled: true,
                ..Default::default()
            },
        );

        assert!(collect_missing_enabled(&settings, |_| false).is_empty());
        let collected = collect_missing(&settings, |_| false);
        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0].name, "OFF_VAR");
    }

    #[test]
    fn toasts_only_when_a_name_is_new() {
        assert!(should_toast(&set(&[]), &set(&["A"])));
        assert!(!should_toast(&set(&["A"]), &set(&["A"])));
        assert!(!should_toast(&set(&["A", "B"]), &set(&["A"])));
        assert!(should_toast(&set(&["A"]), &set(&["A", "B"])));
    }
}
