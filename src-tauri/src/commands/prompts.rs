use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use crate::services::config::{ConfigService, PromptKind};

#[derive(Serialize)]
pub struct PromptDoc {
    pub kind: PromptKind,
    pub label: &'static str,
    pub path: String,
    pub content: String,
    pub supports_env_placeholders: bool,
}

#[derive(Serialize)]
pub struct EnvPlaceholder {
    pub token: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub example: String,
}

#[tauri::command]
pub async fn list_prompts(
    config: State<'_, Arc<Mutex<ConfigService>>>,
) -> crate::Result<Vec<PromptDoc>> {
    let config = config.lock().await;
    Ok(PromptKind::ALL
        .iter()
        .map(|kind| build_prompt_doc(&config, *kind))
        .collect())
}

#[tauri::command]
pub async fn get_prompt(
    config: State<'_, Arc<Mutex<ConfigService>>>,
    kind: PromptKind,
) -> crate::Result<PromptDoc> {
    let config = config.lock().await;
    Ok(build_prompt_doc(&config, kind))
}

#[tauri::command]
pub async fn save_prompt(
    app: AppHandle,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    kind: PromptKind,
    content: String,
) -> crate::Result<()> {
    let config = config.lock().await;
    config.write_prompt(kind, &content)?;
    app.emit("prompt-changed", &kind)?;
    Ok(())
}

#[tauri::command]
pub fn get_environment_placeholders() -> Vec<EnvPlaceholder> {
    let now = chrono::Local::now();
    vec![
        EnvPlaceholder {
            token: "{{date}}",
            label: "Date",
            description: "Local date in YYYY-MM-DD format.",
            example: now.format("%Y-%m-%d").to_string(),
        },
        EnvPlaceholder {
            token: "{{time}}",
            label: "Time",
            description: "Local time in HH:MM (24h) format.",
            example: now.format("%H:%M").to_string(),
        },
        EnvPlaceholder {
            token: "{{timezone}}",
            label: "Timezone",
            description: "System timezone abbreviation.",
            example: now.format("%Z").to_string(),
        },
        EnvPlaceholder {
            token: "{{os}}",
            label: "OS",
            description: "Operating system identifier (linux, macos, windows).",
            example: std::env::consts::OS.to_string(),
        },
        EnvPlaceholder {
            token: "{{active_app}}",
            label: "Active app",
            description: "Foreground application at the moment the conversation started.",
            example: "Firefox".to_string(),
        },
        EnvPlaceholder {
            token: "{{recent_apps}}",
            label: "Recent apps",
            description: "Comma-separated list of recently focused applications.",
            example: "Firefox, VS Code, Slack".to_string(),
        },
    ]
}

fn build_prompt_doc(config: &ConfigService, kind: PromptKind) -> PromptDoc {
    let path = config
        .prompt_path(kind)
        .map(|p| p.to_string())
        .unwrap_or_else(|| kind.default_path().to_string());
    let content = config.read_prompt(kind);
    PromptDoc {
        kind,
        label: kind.label(),
        path,
        content,
        supports_env_placeholders: kind.supports_env_placeholders(),
    }
}
