use serde::{Deserialize, Serialize};

use crate::models::settings::{McpServerConfig, ModelConfig, Settings, WebviewProvider};
use crate::models::skill::SkillFrontmatter;
use crate::services::config::{catalog, PromptKind};

use super::plan::{parse_path, Segment};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptFile {
    System,
    AboutYou,
    Environment,
    InputFormat,
    TitleGeneration,
    SpeechToText,
    SttKeyterms,
}

impl PromptFile {
    pub fn prompt_kind(self) -> Option<PromptKind> {
        match self {
            Self::System => Some(PromptKind::System),
            Self::AboutYou => Some(PromptKind::AboutYou),
            Self::Environment => Some(PromptKind::Environment),
            Self::InputFormat => Some(PromptKind::InputFormat),
            Self::TitleGeneration => Some(PromptKind::TitleGeneration),
            Self::SpeechToText => Some(PromptKind::SpeechToText),
            Self::SttKeyterms => None,
        }
    }

    fn label(self) -> &'static str {
        self.prompt_kind()
            .map_or("Speech-to-text keyterms", |kind| kind.label())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Operation {
    SetSetting { key: String, value: serde_json::Value },
    AddModel { model: ModelConfig },
    UpdateModel { id: String, model: ModelConfig },
    DeleteModel { id: String },
    CreateSkill { slug: String, frontmatter: SkillFrontmatter, body: String, sample_input: String },
    UpdateSkill { slug: String, frontmatter: SkillFrontmatter, body: String, sample_input: String },
    DeleteSkill { slug: String },
    ReorderSkills { order: Vec<String> },
    WritePrompt { kind: PromptFile, content: String },
    AddMcpServer { name: String, config: McpServerConfig },
    UpdateMcpServer { name: String, config: McpServerConfig },
    DeleteMcpServer { name: String },
    AddWebviewProvider { provider: WebviewProvider },
    UpdateWebviewProvider { id: String, provider: WebviewProvider },
    DeleteWebviewProvider { id: String },
    SetShortcut { action: String, shortcut: Option<String> },
    AddEnvVar {
        name: String,
        #[serde(default)]
        description: String,
        #[serde(default)]
        secret: bool,
    },
    UpdateEnvVar {
        name: String,
        #[serde(default)]
        description: String,
        #[serde(default)]
        secret: bool,
    },
    RenameEnvVar { name: String, new_name: String },
    DeleteEnvVar { name: String },
    SetEnvValue { name: String },
}

impl Operation {
    pub const NAMES: [&'static str; 21] = [
        "set_setting",
        "add_model",
        "update_model",
        "delete_model",
        "create_skill",
        "update_skill",
        "delete_skill",
        "reorder_skills",
        "write_prompt",
        "add_mcp_server",
        "update_mcp_server",
        "delete_mcp_server",
        "add_webview_provider",
        "update_webview_provider",
        "delete_webview_provider",
        "set_shortcut",
        "add_env_var",
        "update_env_var",
        "rename_env_var",
        "delete_env_var",
        "set_env_value",
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::SetSetting { .. } => "set_setting",
            Self::AddModel { .. } => "add_model",
            Self::UpdateModel { .. } => "update_model",
            Self::DeleteModel { .. } => "delete_model",
            Self::CreateSkill { .. } => "create_skill",
            Self::UpdateSkill { .. } => "update_skill",
            Self::DeleteSkill { .. } => "delete_skill",
            Self::ReorderSkills { .. } => "reorder_skills",
            Self::WritePrompt { .. } => "write_prompt",
            Self::AddMcpServer { .. } => "add_mcp_server",
            Self::UpdateMcpServer { .. } => "update_mcp_server",
            Self::DeleteMcpServer { .. } => "delete_mcp_server",
            Self::AddWebviewProvider { .. } => "add_webview_provider",
            Self::UpdateWebviewProvider { .. } => "update_webview_provider",
            Self::DeleteWebviewProvider { .. } => "delete_webview_provider",
            Self::SetShortcut { .. } => "set_shortcut",
            Self::AddEnvVar { .. } => "add_env_var",
            Self::UpdateEnvVar { .. } => "update_env_var",
            Self::RenameEnvVar { .. } => "rename_env_var",
            Self::DeleteEnvVar { .. } => "delete_env_var",
            Self::SetEnvValue { .. } => "set_env_value",
        }
    }

    pub fn label(&self, settings: &Settings) -> String {
        match self {
            Self::SetSetting { key, .. } => setting_label(settings, key),
            Self::AddModel { model } => format!("Add model {}", model.display_name),
            Self::UpdateModel { model, .. } => format!("Update model {}", model.display_name),
            Self::DeleteModel { id } => format!("Delete model {}", model_name(settings, id)),
            Self::CreateSkill { slug, .. } => format!("Create skill {slug}"),
            Self::UpdateSkill { slug, .. } => format!("Update skill {slug}"),
            Self::DeleteSkill { slug } => format!("Delete skill {slug}"),
            Self::ReorderSkills { .. } => "Reorder skills".to_string(),
            Self::WritePrompt { kind, .. } => format!("Write {}", kind.label()),
            Self::AddMcpServer { name, .. } => format!("Add MCP server {name}"),
            Self::UpdateMcpServer { name, .. } => format!("Update MCP server {name}"),
            Self::DeleteMcpServer { name } => format!("Delete MCP server {name}"),
            Self::AddWebviewProvider { provider } => format!("Add web provider {}", provider.name),
            Self::UpdateWebviewProvider { provider, .. } => {
                format!("Update web provider {}", provider.name)
            }
            Self::DeleteWebviewProvider { id } => {
                format!("Delete web provider {}", provider_name(settings, id))
            }
            Self::SetShortcut { action, shortcut: Some(_) } => format!("Set shortcut for {action}"),
            Self::SetShortcut { action, shortcut: None } => format!("Clear shortcut for {action}"),
            Self::AddEnvVar { name, .. } => format!("Add variable {name}"),
            Self::UpdateEnvVar { name, .. } => format!("Update variable {name}"),
            Self::RenameEnvVar { name, new_name } => format!("Rename variable {name} to {new_name}"),
            Self::DeleteEnvVar { name } => format!("Delete variable {name}"),
            Self::SetEnvValue { name } => format!("Set value of {name}"),
        }
    }
}

fn model_name<'a>(settings: &'a Settings, id: &'a str) -> &'a str {
    settings
        .models
        .iter()
        .find(|model| model.id == id)
        .map_or(id, |model| model.display_name.as_str())
}

fn provider_name<'a>(settings: &'a Settings, id: &'a str) -> &'a str {
    settings
        .webview_providers
        .iter()
        .find(|provider| provider.id == id)
        .map_or(id, |provider| provider.name.as_str())
}

fn setting_label(settings: &Settings, key: &str) -> String {
    let catalog_key = catalog::catalog_path(key);
    let Some(entry) = catalog::ENTRIES.iter().find(|entry| entry.key == catalog_key) else {
        return key.to_string();
    };
    if catalog_key == key {
        return entry.label.to_string();
    }
    let item = match parse_path(key).as_slice() {
        [Segment::Key(list), Segment::Item(id), ..] if list == "models" => {
            model_name(settings, id).to_string()
        }
        [Segment::Key(list), Segment::Item(id), ..] if list == "webview_providers" => {
            provider_name(settings, id).to_string()
        }
        _ => return key.to_string(),
    };
    format!("{item}: {}", entry.label)
}
